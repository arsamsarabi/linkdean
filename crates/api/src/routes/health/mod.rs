use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(ready))
}

/// Liveness. Reports the running version so a deploy can confirm which build is live.
#[utoipa::path(
    get,
    operation_id = "health_health",
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "linkdean",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Liveness of dependencies (Postgres + Meilisearch). Use for Compose / k8s probes.
#[utoipa::path(
    get,
    operation_id = "health_ready",
    path = "/ready",
    tag = "ready",
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let mut db_ok = false;
    let mut meili_ok = false;

    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => db_ok = true,
        Err(e) => tracing::warn!(error = %e, "ready: database check failed"),
    }

    let meili_url = state.cfg.meili_url.trim_end_matches('/');
    match reqwest::Client::new()
        .get(format!("{meili_url}/health"))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => meili_ok = true,
        Ok(res) => tracing::warn!(status = %res.status(), "ready: meilisearch check failed"),
        Err(e) => tracing::warn!(error = %e, "ready: meilisearch check failed"),
    }

    let body = json!({
        "status": if db_ok && meili_ok { "ready" } else { "degraded" },
        "service": "linkdean",
        "checks": {
            "db": db_ok,
            "meilisearch": meili_ok,
        },
    });

    if db_ok && meili_ok {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_reports_the_crate_version() {
        let Json(body) = health().await;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    }
}
