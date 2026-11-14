//! Caching headers for `/api`.
//!
//! A successful `GET` or `HEAD` that returns JSON gets a strong `ETag` (a hash of the body)
//! and `Cache-Control: private, no-cache`: a browser may keep the answer but must ask again
//! each time, and a matching `If-None-Match` gets `304 Not Modified` with no body. Shared
//! caches never store it, because answers depend on who is signed in. Everything else under
//! `/api` that has not set its own `Cache-Control` (writes, errors, downloads) gets
//! `no-store`. Routes that set `Cache-Control` themselves, such as images, keep theirs.

use axum::body::{Body, HttpBody};
use axum::extract::Request;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use base64::Engine;
use sha2::{Digest, Sha256};

/// The `Cache-Control` of a cacheable JSON read.
pub const REVALIDATE: &str = "private, no-cache";
/// The `Cache-Control` of everything else.
pub const NO_STORE: &str = "no-store";
/// Larger JSON bodies are sent without an `ETag` rather than buffered.
const MAX_ETAG_BODY: u64 = 4 * 1024 * 1024;

pub async fn caching_headers(req: Request, next: Next) -> Response {
    let readable = matches!(*req.method(), Method::GET | Method::HEAD);
    let if_none_match = req.headers().get(header::IF_NONE_MATCH).cloned();
    let res = next.run(req).await;
    if res.headers().contains_key(header::CACHE_CONTROL) {
        return res;
    }
    if !readable || res.status() != StatusCode::OK || !cacheable_json(res.headers()) {
        return with_cache_control(res, NO_STORE);
    }
    let Some(len) = res.body().size_hint().exact() else {
        return with_cache_control(res, NO_STORE);
    };
    if len > MAX_ETAG_BODY {
        return with_cache_control(res, REVALIDATE);
    }
    let (mut parts, body) = res.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MAX_ETAG_BODY as usize).await else {
        // The body failed mid-read; there is nothing left to send.
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap_or_default();
    };
    let etag = etag_for(&bytes);
    parts
        .headers
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(REVALIDATE));
    if let Ok(v) = HeaderValue::from_str(&etag) {
        parts.headers.insert(header::ETAG, v);
    }
    if if_none_match.is_some_and(|v| matches_etag(&v, &etag)) {
        parts.status = StatusCode::NOT_MODIFIED;
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.remove(header::CONTENT_TYPE);
        return Response::from_parts(parts, Body::empty());
    }
    Response::from_parts(parts, Body::from(bytes))
}

/// JSON that is shown, not downloaded.
fn cacheable_json(headers: &HeaderMap) -> bool {
    let json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    json && !headers.contains_key(header::CONTENT_DISPOSITION)
}

fn with_cache_control(mut res: Response, value: &'static str) -> Response {
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    res
}

/// A strong `ETag`: the first 128 bits of the body's SHA-256, base64url.
pub fn etag_for(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    format!(
        "\"{}\"",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&digest[..16])
    )
}

/// `If-None-Match` matches `*` or any listed tag, weak or strong (RFC 9110 weak comparison).
fn matches_etag(if_none_match: &HeaderValue, etag: &str) -> bool {
    let Ok(v) = if_none_match.to_str() else {
        return false;
    };
    v.split(',')
        .map(str::trim)
        .any(|t| t == "*" || t.strip_prefix("W/").unwrap_or(t) == etag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde_json::json;
    use tower::ServiceExt;

    fn app() -> Router {
        Router::new()
            .route("/items", get(|| async { Json(json!({ "items": [1, 2] })) }))
            .route("/items", post(|| async { Json(json!({ "ok": true })) }))
            .route(
                "/image",
                get(|| async { ([(header::CACHE_CONTROL, "public, max-age=60")], "png-bytes") }),
            )
            .route(
                "/export",
                get(|| async {
                    (
                        [(header::CONTENT_DISPOSITION, "attachment; filename=x.json")],
                        Json(json!({ "all": true })),
                    )
                }),
            )
            .route(
                "/missing",
                get(|| async { (StatusCode::NOT_FOUND, Json(json!({ "code": "not_found" }))) }),
            )
            .layer(axum::middleware::from_fn(caching_headers))
    }

    async fn send(method: &str, path: &str, inm: Option<&str>) -> Response {
        let mut b = axum::http::Request::builder().method(method).uri(path);
        if let Some(v) = inm {
            b = b.header(header::IF_NONE_MATCH, v);
        }
        app().oneshot(b.body(Body::empty()).unwrap()).await.unwrap()
    }

    #[tokio::test]
    async fn json_reads_get_an_etag_and_revalidate() {
        let res = send("GET", "/items", None).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CACHE_CONTROL], REVALIDATE);
        let etag = res.headers()[header::ETAG].to_str().unwrap().to_string();
        assert!(etag.starts_with('"') && etag.ends_with('"'));

        let again = send("GET", "/items", Some(&etag)).await;
        assert_eq!(again.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(again.headers()[header::ETAG], etag.as_str());
        let body = axum::body::to_bytes(again.into_body(), 1024).await.unwrap();
        assert!(body.is_empty());

        let weak = send("GET", "/items", Some(&format!("\"other\", W/{etag}"))).await;
        assert_eq!(weak.status(), StatusCode::NOT_MODIFIED);

        let stale = send("GET", "/items", Some("\"stale\"")).await;
        assert_eq!(stale.status(), StatusCode::OK);
        let body = axum::body::to_bytes(stale.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], br#"{"items":[1,2]}"#);
    }

    #[tokio::test]
    async fn writes_errors_and_downloads_are_not_stored() {
        for (m, p) in [("POST", "/items"), ("GET", "/missing"), ("GET", "/export")] {
            let res = send(m, p, None).await;
            assert_eq!(res.headers()[header::CACHE_CONTROL], NO_STORE, "{m} {p}");
            assert!(!res.headers().contains_key(header::ETAG), "{m} {p}");
        }
    }

    #[tokio::test]
    async fn routes_with_their_own_cache_control_keep_it() {
        let res = send("GET", "/image", None).await;
        assert_eq!(res.headers()[header::CACHE_CONTROL], "public, max-age=60");
        assert!(!res.headers().contains_key(header::ETAG));
    }

    #[test]
    fn etag_is_stable_and_body_specific() {
        assert_eq!(etag_for(b"a"), etag_for(b"a"));
        assert_ne!(etag_for(b"a"), etag_for(b"b"));
    }
}
