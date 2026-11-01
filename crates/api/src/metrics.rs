//! Prometheus metrics: HTTP request counters / latency histograms + process metrics.

use crate::state::AppState;
use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
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

/// Collapse dynamic path segments so cardinality stays bounded.
fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".into();
    }
    trimmed
        .split('/')
        .map(|seg| {
            if seg.is_empty() {
                ""
            } else if looks_like_id(seg) {
                ":id"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn looks_like_id(seg: &str) -> bool {
    if seg.len() == 36 && seg.chars().filter(|c| *c == '-').count() == 4 {
        return true;
    }
    if seg.chars().all(|c| c.is_ascii_digit()) && seg.len() > 3 {
        return true;
    }
    false
}

pub async fn track_http(req: Request, next: Next) -> Response {
    let method = req.method().as_str().to_string();
    let path = normalize_path(req.uri().path());
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

async fn metrics_handler() -> impl IntoResponse {
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

    #[test]
    fn normalize_uuid_segments() {
        assert_eq!(
            normalize_path("/bookmarks/550e8400-e29b-41d4-a716-446655440000"),
            "/bookmarks/:id"
        );
        assert_eq!(normalize_path("/health"), "/health");
        assert_eq!(normalize_path("/"), "/");
    }
}
