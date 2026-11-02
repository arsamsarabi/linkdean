//! Prometheus metrics: HTTP request counters / latency histograms + process metrics.

use crate::state::AppState;
use axum::body::Body;
use axum::extract::{MatchedPath, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};
use std::sync::OnceLock;
use std::time::Instant;

static REGISTRY: OnceLock<Registry> = OnceLock::new();
static HTTP_REQUESTS: OnceLock<IntCounterVec> = OnceLock::new();
static HTTP_DURATION: OnceLock<HistogramVec> = OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let reg = Registry::new();
        // Process / Rust runtime collectors (CPU, RSS, threads, …). Linux-only in `prometheus`.
        #[cfg(target_os = "linux")]
        let _ = reg.register(Box::new(
            prometheus::process_collector::ProcessCollector::for_self(),
        ));
        reg
    })
}

fn http_requests() -> &'static IntCounterVec {
    HTTP_REQUESTS.get_or_init(|| {
        let c = IntCounterVec::new(
            Opts::new(
                "linkdean_http_requests_total",
                "Total HTTP requests handled by the Linkdean API",
            ),
            &["method", "path", "status"],
        )
        .expect("http requests counter");
        registry()
            .register(Box::new(c.clone()))
            .expect("register http requests");
        c
    })
}

fn http_duration() -> &'static HistogramVec {
    HTTP_DURATION.get_or_init(|| {
        let h = HistogramVec::new(
            HistogramOpts::new(
                "linkdean_http_request_duration_seconds",
                "HTTP request latency in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "path", "status"],
        )
        .expect("http duration histogram");
        registry()
            .register(Box::new(h.clone()))
            .expect("register http duration");
        h
    })
}

/// Ensure metric series exist before the first scrape.
pub fn init() {
    let _ = http_requests();
    let _ = http_duration();
}

/// The route template, such as `/api/l/{slug}`, never the raw path. Raw paths would put
/// public-list slugs in the labels and let random paths grow memory without limit.
fn route_label(req: &Request) -> String {
    req.extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unmatched".into())
}

pub async fn track_http(req: Request, next: Next) -> Response {
    let method = req.method().as_str().to_string();
    let path = route_label(&req);
    let start = Instant::now();
    let res = next.run(req).await;
    let status = res.status().as_u16().to_string();
    let elapsed = start.elapsed().as_secs_f64();
    http_requests()
        .with_label_values(&[&method, &path, &status])
        .inc();
    http_duration()
        .with_label_values(&[&method, &path, &status])
        .observe(elapsed);
    res
}

/// Constant-time check of `Authorization: Bearer <token>`.
fn bearer_matches(headers: &HeaderMap, token: &str) -> bool {
    let Some(given) = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return false;
    };
    let (a, b) = (given.as_bytes(), token.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn metrics_handler(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match state.cfg.metrics_token.as_deref() {
        None => return StatusCode::NOT_FOUND.into_response(),
        Some(token) if !bearer_matches(&headers, token) => {
            return StatusCode::UNAUTHORIZED.into_response()
        }
        Some(_) => {}
    }
    let encoder = TextEncoder::new();
    let metric_families = registry().gather();
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("encode metrics: {e}"),
        )
            .into_response();
    }
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            encoder.format_type().to_string(),
        )],
        Body::from(buffer),
    )
        .into_response()
}

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use tower::ServiceExt;

    #[tokio::test]
    async fn labels_use_the_route_template() {
        let app: Router = Router::new()
            .route("/l/{slug}", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(track_http));
        let req = Request::builder()
            .uri("/l/secret-list-slug-label-test")
            .body(Body::empty())
            .expect("request");
        app.clone().oneshot(req).await.expect("response");
        let req = Request::builder()
            .uri("/random-path-label-test")
            .body(Body::empty())
            .expect("request");
        app.oneshot(req).await.expect("response");

        let mut out = Vec::new();
        TextEncoder::new()
            .encode(&registry().gather(), &mut out)
            .expect("encode");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains(r#"path="/l/{slug}""#));
        assert!(!text.contains("secret-list-slug-label-test"));
        assert!(!text.contains("random-path-label-test"));
    }

    #[test]
    fn bearer_must_match_exactly() {
        let mut headers = HeaderMap::new();
        assert!(!bearer_matches(&headers, "token-abc"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token-abd"));
        assert!(!bearer_matches(&headers, "token-abc"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token-ab"));
        assert!(!bearer_matches(&headers, "token-abc"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token-abc"));
        assert!(bearer_matches(&headers, "token-abc"));
    }
}
