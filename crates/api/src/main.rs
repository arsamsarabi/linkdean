mod auth;
mod config;
mod error;
mod routes;
mod state;

use axum::Router;
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
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

    let api = Router::new()
        .merge(routes::router())
        .with_state(state);

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "apps/web/dist".into());
    let index = format!("{static_dir}/index.html");
    let app = Router::new()
        .nest("/api", api)
        .fallback_service(
            ServeDir::new(&static_dir).not_found_service(ServeFile::new(index)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = cfg.bind.parse()?;
    tracing::info!("linkdean listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
