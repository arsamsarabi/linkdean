//! The one error body every `/api` route answers with:
//!
//! ```json
//! { "code": "not_found", "message": "not found", "details": { … }, "error": "not found" }
//! ```
//!
//! `code` is a stable, machine-readable id; `message` is for people and may change. `details`
//! appears only when the error carries data (a ban, a quota, what blocked a move). `error`
//! repeats `message` for clients written before `code` and `message` existed, such as browser
//! extension builds already in the stores.
//!
//! Errors raised outside the handlers (axum's own extractor rejections, an unknown method, a
//! body over the size limit) are rewritten into the same shape by [`normalize_error_body`].

use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use linkdean_core::Error as CoreError;
use serde::Serialize;
use serde_json::{json, Value};

pub struct ApiError(pub CoreError);

impl From<CoreError> for ApiError {
    fn from(value: CoreError) -> Self {
        Self(value)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self(CoreError::msg(value.to_string()))
    }
}

/// The error body. See the module docs.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    /// Stable machine-readable id, such as `not_found` or `quota_exceeded`.
    pub code: String,
    /// Human-readable text. Wording may change between releases; match on `code`.
    pub message: String,
    /// Extra data for some codes: `ban` for `banned`, `meter`/`limit`/`used` for
    /// `quota_exceeded`, `blockers` for `vault_move_blocked`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub details: Option<Value>,
    /// Same as `message`. Kept for older clients.
    pub error: String,
}

impl ErrorBody {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: code.into(),
            error: message.clone(),
            message,
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    /// The response with `status`.
    pub fn into_response_with(self, status: StatusCode) -> Response {
        (status, Json(self)).into_response()
    }
}

/// Status, code and message for a core error. Internal errors are logged here and never
/// leak their text.
fn describe(e: &CoreError) -> (StatusCode, &'static str, String) {
    match e {
        CoreError::Banned { .. } => (StatusCode::FORBIDDEN, "banned", e.to_string()),
        CoreError::EmailUnverified => (StatusCode::FORBIDDEN, "email_unverified", e.to_string()),
        CoreError::QuotaExceeded { .. } => (StatusCode::FORBIDDEN, "quota_exceeded", e.to_string()),
        CoreError::SessionExpired => (StatusCode::UNAUTHORIZED, "session_expired", e.to_string()),
        CoreError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", e.to_string()),
        CoreError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", e.to_string()),
        CoreError::NotFound => (StatusCode::NOT_FOUND, "not_found", e.to_string()),
        CoreError::Conflict(m) => (StatusCode::CONFLICT, "conflict", m.clone()),
        CoreError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
        CoreError::VaultLocked => (StatusCode::LOCKED, "vault_locked", e.to_string()),
        CoreError::VaultUnlockFailed => (
            StatusCode::UNAUTHORIZED,
            "vault_unlock_failed",
            e.to_string(),
        ),
        CoreError::TooManyRequests => (
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_requests",
            e.to_string(),
        ),
        CoreError::Msg(m) => {
            tracing::error!(error = %m, "internal error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal error".into(),
            )
        }
        CoreError::Other(err) => {
            tracing::error!(error = %err, "internal error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal error".into(),
            )
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = describe(&self.0);
        let mut body = ErrorBody::new(code, message);
        match &self.0 {
            CoreError::Banned { reason, ends_at } => {
                body =
                    body.with_details(json!({ "ban": { "reason": reason, "ends_at": ends_at } }));
            }
            CoreError::QuotaExceeded { meter, limit, used } => {
                body = body.with_details(json!({ "meter": meter, "limit": limit, "used": used }));
            }
            _ => {}
        }
        body.into_response_with(status)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

/// The `code` for an error status that did not come from [`ApiError`].
fn code_for_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::CONFLICT => "conflict",
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
        StatusCode::UNPROCESSABLE_ENTITY => "invalid_body",
        StatusCode::LOCKED => "vault_locked",
        StatusCode::TOO_MANY_REQUESTS => "too_many_requests",
        StatusCode::SERVICE_UNAVAILABLE => "unavailable",
        s if s.is_server_error() => "internal",
        _ => "error",
    }
}

/// Longest rejection text copied into `message`.
const MAX_REJECTION_MESSAGE: usize = 300;

/// Rewrites any `/api` error response that is not already JSON (axum's extractor rejections,
/// 405s, bodies over the limit, bare status codes) into an [`ErrorBody`]. JSON error bodies
/// pass through untouched.
pub async fn normalize_error_body(req: Request, next: Next) -> Response {
    let res = next.run(req).await;
    let status = res.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return res;
    }
    let is_json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if is_json {
        return res;
    }
    let (mut parts, body) = res.into_parts();
    let text = axum::body::to_bytes(body, 64 * 1024)
        .await
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
        .unwrap_or_default();
    let message = if text.is_empty() || status.is_server_error() {
        status
            .canonical_reason()
            .unwrap_or("error")
            .to_ascii_lowercase()
    } else {
        text.chars().take(MAX_REJECTION_MESSAGE).collect()
    };
    let json =
        serde_json::to_vec(&ErrorBody::new(code_for_status(status), message)).unwrap_or_default();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Response::from_parts(parts, Body::from(json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    async fn body_json(res: Response) -> Value {
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn every_core_error_has_code_message_and_legacy_error() {
        for (e, status, code) in [
            (CoreError::NotFound, 404, "not_found"),
            (CoreError::Unauthorized, 401, "unauthorized"),
            (CoreError::SessionExpired, 401, "session_expired"),
            (CoreError::Forbidden, 403, "forbidden"),
            (CoreError::Conflict("taken".into()), 409, "conflict"),
            (CoreError::BadRequest("bad url".into()), 400, "bad_request"),
            (CoreError::VaultLocked, 423, "vault_locked"),
            (CoreError::VaultUnlockFailed, 401, "vault_unlock_failed"),
            (CoreError::TooManyRequests, 429, "too_many_requests"),
            (CoreError::EmailUnverified, 403, "email_unverified"),
            (CoreError::msg("db exploded at 10.0.0.3"), 500, "internal"),
        ] {
            let res = ApiError(e).into_response();
            assert_eq!(res.status().as_u16(), status, "{code}");
            let v = body_json(res).await;
            assert_eq!(v["code"], code);
            assert!(v["message"].is_string());
            assert_eq!(v["error"], v["message"]);
            assert!(v.get("details").is_none());
        }
    }

    #[tokio::test]
    async fn internal_errors_hide_their_text() {
        let v = body_json(ApiError(CoreError::msg("secret path /etc/x")).into_response()).await;
        assert_eq!(v["message"], "internal error");
    }

    #[tokio::test]
    async fn bans_and_quotas_put_their_data_in_details() {
        let v = body_json(
            ApiError(CoreError::Banned {
                reason: "spam".into(),
                ends_at: None,
            })
            .into_response(),
        )
        .await;
        assert_eq!(v["code"], "banned");
        assert_eq!(v["details"]["ban"]["reason"], "spam");
        let v = body_json(
            ApiError(CoreError::QuotaExceeded {
                meter: "bookmarks",
                limit: 10,
                used: 10,
            })
            .into_response(),
        )
        .await;
        assert_eq!(v["code"], "quota_exceeded");
        assert_eq!(v["details"]["meter"], "bookmarks");
        assert_eq!(v["details"]["limit"], 10);
    }

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Input {
        name: String,
    }

    fn app() -> Router {
        Router::new()
            .route("/thing", post(|Json(_): Json<Input>| async { "ok" }))
            .route("/bare", get(|| async { StatusCode::UNAUTHORIZED }))
            .layer(axum::middleware::from_fn(normalize_error_body))
    }

    async fn call(req: axum::http::Request<Body>) -> (u16, Value) {
        let res = app().oneshot(req).await.unwrap();
        let status = res.status().as_u16();
        assert_eq!(
            res.headers()[header::CONTENT_TYPE],
            "application/json",
            "status {status}"
        );
        (status, body_json(res).await)
    }

    #[tokio::test]
    async fn axum_rejections_become_error_bodies() {
        let req = |ct: &str, body: &str| {
            axum::http::Request::post("/thing")
                .header(header::CONTENT_TYPE, ct)
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let (s, v) = call(req("application/json", "{\"name\": 3}")).await;
        assert_eq!((s, v["code"].as_str()), (422, Some("invalid_body")));
        assert!(v["message"].as_str().unwrap().contains("name"));
        let (s, v) = call(req("application/json", "{")).await;
        assert_eq!((s, v["code"].as_str()), (400, Some("bad_request")));
        let (s, v) = call(req("text/plain", "x")).await;
        assert_eq!(
            (s, v["code"].as_str()),
            (415, Some("unsupported_media_type"))
        );
        let (s, v) = call(
            axum::http::Request::get("/thing")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!((s, v["code"].as_str()), (405, Some("method_not_allowed")));
        let (s, v) = call(
            axum::http::Request::get("/bare")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!((s, v["code"].as_str()), (401, Some("unauthorized")));
        assert_eq!(v["message"], "unauthorized");
    }
}
