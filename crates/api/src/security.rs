//! App-wide security headers and `security.txt`.

use axum::extract::Request;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

/// SPA-safe CSP. Image asset routes set their own sandbox CSP; middleware keeps that.
/// `challenges.cloudflare.com` is the Turnstile script and iframe on registration.
pub const CONTENT_SECURITY_POLICY: &str = "\
default-src 'self'; \
script-src 'self' https://challenges.cloudflare.com; \
frame-src https://challenges.cloudflare.com; \
style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob:; \
font-src 'self' data:; \
connect-src 'self'; \
worker-src 'self' blob:; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'self'; \
object-src 'none'";

const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=(), usb=()";

/// Build the baseline security headers. `hsts_max_age` is seconds; `None` skips HSTS.
pub fn security_headers(hsts_max_age: Option<u64>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(PERMISSIONS_POLICY),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY),
    );
    if let Some(secs) = hsts_max_age {
        let value = format!("max-age={secs}; includeSubDomains");
        if let Ok(hv) = HeaderValue::from_str(&value) {
            headers.insert(header::STRICT_TRANSPORT_SECURITY, hv);
        }
    }
    headers
}

/// Apply baseline headers to a response. Does not replace an existing CSP (asset sandbox).
pub fn apply_security_headers(headers: &mut HeaderMap, hsts_max_age: Option<u64>) {
    let baseline = security_headers(hsts_max_age);
    for (name, value) in baseline.iter() {
        if name == header::CONTENT_SECURITY_POLICY && headers.contains_key(name) {
            continue;
        }
        headers.insert(name.clone(), value.clone());
    }
}

pub async fn security_headers_middleware(
    hsts_max_age: Option<u64>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    apply_security_headers(response.headers_mut(), hsts_max_age);
    response
}

/// RFC 9116 body. Bump `Expires` when it nears.
pub const SECURITY_TXT: &str = "\
Contact: mailto:arsamsarabi@me.com\n\
Preferred-Languages: en\n\
Canonical: https://linkdean.app/.well-known/security.txt\n\
Expires: 2027-11-05T00:00:00.000Z\n\
";

async fn security_txt() -> Response {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        SECURITY_TXT,
    )
        .into_response()
}

/// Routes for `/.well-known/security.txt` and `/security.txt` (outer router, before SPA).
pub fn security_txt_router() -> Router {
    Router::new()
        .route("/.well-known/security.txt", get(security_txt))
        .route("/security.txt", get(security_txt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get as get_route;
    use tower::ServiceExt;

    #[test]
    fn baseline_headers_cover_the_roadmap() {
        let h = security_headers(None);
        assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(
            h[header::REFERRER_POLICY],
            "strict-origin-when-cross-origin"
        );
        assert_eq!(h[header::X_FRAME_OPTIONS], "DENY");
        assert!(h[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'"));
        assert!(h[HeaderName::from_static("permissions-policy")]
            .to_str()
            .unwrap()
            .contains("camera=()"));
        assert!(!h.contains_key(header::STRICT_TRANSPORT_SECURITY));
    }

    #[test]
    fn csp_allows_the_turnstile_widget() {
        let csp = CONTENT_SECURITY_POLICY;
        assert!(csp.contains("script-src 'self' https://challenges.cloudflare.com;"));
        assert!(csp.contains("frame-src https://challenges.cloudflare.com;"));
    }

    #[test]
    fn hsts_is_optional() {
        let h = security_headers(Some(31_536_000));
        assert_eq!(
            h[header::STRICT_TRANSPORT_SECURITY],
            "max-age=31536000; includeSubDomains"
        );
    }

    #[test]
    fn apply_keeps_an_existing_csp() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("sandbox; default-src 'none'"),
        );
        apply_security_headers(&mut headers, None);
        assert_eq!(
            headers[header::CONTENT_SECURITY_POLICY],
            "sandbox; default-src 'none'"
        );
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    }

    #[tokio::test]
    async fn security_txt_lists_the_project_contact() {
        let app = security_txt_router();
        for path in ["/.well-known/security.txt", "/security.txt"] {
            let res = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
            let text = String::from_utf8(body.to_vec()).unwrap();
            assert!(text.contains("Contact: mailto:arsamsarabi@me.com"));
            assert!(text.contains("Expires:"));
        }
    }

    #[tokio::test]
    async fn middleware_adds_headers_to_a_plain_response() {
        let app = Router::new()
            .route("/ping", get_route(|| async { "ok" }))
            .layer(axum::middleware::from_fn(|req, next| async move {
                security_headers_middleware(None, req, next).await
            }));
        let res = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert!(res.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("frame-ancestors 'none'"));
    }
}
