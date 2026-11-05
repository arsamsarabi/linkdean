use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use linkdean_core::Error as CoreError;
use serde_json::json;

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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let CoreError::Banned { reason, ends_at } = &self.0 {
            let body = json!({
                "error": "banned",
                "code": "banned",
                "ban": { "reason": reason, "ends_at": ends_at },
            });
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        if matches!(self.0, CoreError::EmailUnverified) {
            let body = json!({ "error": "email_unverified", "code": "email_unverified" });
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        if matches!(self.0, CoreError::SessionExpired) {
            let body = json!({ "error": self.0.to_string(), "code": "session_expired" });
            return (StatusCode::UNAUTHORIZED, Json(body)).into_response();
        }
        if let CoreError::QuotaExceeded { meter, limit, used } = &self.0 {
            let body = json!({
                "error": "quota_exceeded",
                "code": "quota_exceeded",
                "meter": meter,
                "limit": limit,
                "used": used,
            });
            return (StatusCode::FORBIDDEN, Json(body)).into_response();
        }
        let (status, msg) = match &self.0 {
            CoreError::Banned { .. } => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::EmailUnverified => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::QuotaExceeded { .. } => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::SessionExpired => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::Unauthorized => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::Forbidden => (StatusCode::FORBIDDEN, self.0.to_string()),
            CoreError::NotFound => (StatusCode::NOT_FOUND, self.0.to_string()),
            CoreError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            CoreError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            CoreError::VaultLocked => (StatusCode::LOCKED, self.0.to_string()),
            CoreError::VaultUnlockFailed => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            CoreError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, self.0.to_string()),
            CoreError::Msg(m) => {
                tracing::error!(error = %m, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
            CoreError::Other(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
