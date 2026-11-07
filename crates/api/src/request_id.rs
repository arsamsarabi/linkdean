//! HTTP `X-Request-Id`: validate inbound values, mint UUIDs, correlate into trace spans.

use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderName;
use axum::middleware::Next;
use axum::response::Response;
use tower_http::request_id::RequestId;
use tower_http::trace::MakeSpan;

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

const MAX_LEN: usize = 128;

/// Characters allowed in an inbound request id (ASCII only).
pub fn is_valid_request_id(value: &str) -> bool {
    let len = value.len();
    (1..=MAX_LEN).contains(&len)
        && value
            .bytes()
            .all(|b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.'))
}

/// Drop an invalid inbound `X-Request-Id` so [`SetRequestId`] can mint a replacement.
pub async fn normalize_request_id(mut req: Request, next: Next) -> Response {
    if let Some(raw) = req.headers().get(&X_REQUEST_ID) {
        let ok = raw.to_str().ok().is_some_and(is_valid_request_id);
        if !ok {
            req.headers_mut().remove(&X_REQUEST_ID);
        }
    }
    next.run(req).await
}

#[derive(Clone, Copy, Debug)]
pub struct RequestIdMakeSpan;

impl MakeSpan<Body> for RequestIdMakeSpan {
    fn make_span(&mut self, request: &Request) -> tracing::Span {
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .and_then(|id| id.header_value().to_str().ok())
            .unwrap_or("-");
        tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
            request_id = %request_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;
    use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

    fn app() -> Router {
        Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(axum::middleware::from_fn(normalize_request_id))
    }

    #[test]
    fn accepts_uuid_and_simple_tokens() {
        assert!(is_valid_request_id("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_request_id("cf-ray-abc_123.def"));
    }

    #[test]
    fn rejects_empty_spaces_and_overlong() {
        assert!(!is_valid_request_id(""));
        assert!(!is_valid_request_id("bad id"));
        assert!(!is_valid_request_id(&"a".repeat(129)));
        assert!(!is_valid_request_id("has/slash"));
    }

    #[tokio::test]
    async fn response_echoes_valid_inbound_id() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .header(&X_REQUEST_ID, "proxy-trace-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.headers()[&X_REQUEST_ID], "proxy-trace-1");
    }

    #[tokio::test]
    async fn response_mints_when_missing() {
        let res = app()
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let id = res.headers()[&X_REQUEST_ID].to_str().unwrap();
        assert!(is_valid_request_id(id));
        assert_eq!(id.len(), 36);
    }

    #[tokio::test]
    async fn response_replaces_invalid_inbound_id() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .header(&X_REQUEST_ID, "not a valid id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = res.headers()[&X_REQUEST_ID].to_str().unwrap();
        assert!(is_valid_request_id(id));
        assert_ne!(id, "not a valid id");
    }
}
