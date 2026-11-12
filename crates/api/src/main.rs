mod auth;
mod config;
mod disposable_email;
mod error;
mod logging;
mod mail;
mod maintenance;
mod metrics;
mod notification_mail;
mod quotas;
mod rate_limit;
mod request_id;
mod routes;
mod security;
mod shutdown;
mod state;
mod turnstile;
mod vault_keys;

use axum::http::{HeaderValue, Method};
use axum::Router;
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    logging::init()?;

    let cfg = config::Config::from_env()?;
    let state = AppState::new(cfg.clone()).await?;

    tokio::spawn(maintenance::run(state.clone()));
    tokio::spawn(mail::run_sender(state.clone()));

    // Chrome has no route out of its network; this proxy is its only way to the web.
    if let (Some(addr), Some(_)) = (cfg.browser_proxy_bind, &cfg.browser_ws_url) {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tokio::spawn(linkdean_worker::egress_proxy::serve(
            listener,
            linkdean_worker::egress_proxy::public_resolver(),
        ));
    }

    {
        let pool = state.pool.clone();
        let search = state.search.clone();
        let opts = state.enrich_opts();
        if opts.browser_ws_url.is_some() {
            tokio::spawn(linkdean_worker::run_preview_backfill(pool, search, opts));
        }
    }

    {
        let pool = state.pool.clone();
        let opts = state.cfg.link_check.clone();
        tokio::spawn(async move {
            linkdean_worker::run_link_checker(pool, opts).await;
        });
    }

    metrics::init();
    let api = Router::new()
        .merge(routes::api_router(cfg.dev_routes))
        .merge(metrics::router())
        // Without its own fallback, /api would inherit the app shell below.
        .fallback(api_not_found)
        .layer(axum::middleware::from_fn(metrics::track_http))
        .with_state(state);

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "apps/web/dist".into());
    let index = format!("{static_dir}/index.html");
    let cors = build_cors(&cfg.cors_origins)?;
    let hsts_max_age = cfg.hsts_max_age;
    let csp = security::content_security_policy(cfg.goatcounter_url.as_deref());
    let app = Router::new()
        .merge(security::security_txt_router())
        .nest("/api", api)
        // Client routes such as /library are the app shell, with 200 like any page.
        .fallback_service(ServeDir::new(&static_dir).fallback(ServeFile::new(index)))
        .layer(cors)
        .layer(axum::middleware::from_fn(move |req, next| {
            let hsts = hsts_max_age;
            let csp = csp.clone();
            async move { security::security_headers_middleware(hsts, csp, req, next).await }
        }))
        // Last `.layer` is outermost: normalize → set id → trace → propagate → security → cors.
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TraceLayer::new_for_http().make_span_with(request_id::RequestIdMakeSpan))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(axum::middleware::from_fn(request_id::normalize_request_id));

    let addr: SocketAddr = cfg.bind.parse()?;
    tracing::info!("linkdean listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    use std::future::IntoFuture;
    let stopping = std::sync::Arc::new(tokio::sync::Notify::new());
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal(stopping.clone()))
        .into_future();
    match shutdown::drain_with_cap(server, stopping, shutdown::DRAIN_CAP).await {
        Some(result) => result?,
        None => tracing::warn!(
            cap_secs = shutdown::DRAIN_CAP.as_secs(),
            "requests still open at the shutdown cap; exiting anyway"
        ),
    }
    tracing::info!("linkdean stopped");
    Ok(())
}

async fn api_not_found() -> error::ApiError {
    linkdean_core::Error::NotFound.into()
}

fn build_cors(origins: &[String]) -> anyhow::Result<CorsLayer> {
    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);
    if origins.iter().any(|o| o == "*") {
        tracing::warn!("CORS_ORIGINS=* — allowing any origin (dev/homelab tradeoff)");
        return Ok(layer.allow_origin(Any));
    }
    let parsed: Result<Vec<HeaderValue>, _> =
        origins.iter().map(|o| o.parse::<HeaderValue>()).collect();
    let parsed = parsed.map_err(|e| anyhow::anyhow!("invalid CORS_ORIGINS: {e}"))?;
    Ok(layer.allow_origin(AllowOrigin::list(parsed)))
}
