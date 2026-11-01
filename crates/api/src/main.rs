mod auth;
mod config;
mod error;
mod metrics;
mod routes;
mod state;

use axum::http::{HeaderValue, Method};
use axum::Router;
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = config::Config::from_env()?;
    let state = AppState::new(cfg.clone()).await?;

    {
        let pool = state.pool.clone();
        match linkdean_db::purge_expired_trash(&pool).await {
            Ok((b, c, d)) if b + c + d > 0 => {
                tracing::info!(bookmarks = b, categories = c, dashboards = d, "purged expired trash");
            }
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "trash retention purge failed"),
        }
    }

    {
        let pool = state.pool.clone();
        let search = state.search.clone();
        let opts = state.enrich_opts();
        if opts.browser_ws_url.is_some() {
            tokio::spawn(async move {
                // Give Chrome a moment when compose starts together.
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                linkdean_worker::backfill_previews(pool, search, opts).await;
            });
        }
    }

    metrics::init();
    let api = Router::new()
        .merge(routes::router())
        .merge(metrics::router())
        .layer(axum::middleware::from_fn(metrics::track_http))
        .with_state(state);

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "apps/web/dist".into());
    let index = format!("{static_dir}/index.html");
    let cors = build_cors(&cfg.cors_origins)?;
    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(&static_dir).not_found_service(ServeFile::new(index)))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = cfg.bind.parse()?;
    tracing::info!("linkdean listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
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
