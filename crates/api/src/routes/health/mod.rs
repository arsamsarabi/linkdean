use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "linkdean" }))
}

/// Liveness of dependencies (Postgres + Meilisearch). Use for Compose / k8s probes.
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let mut db_ok = false;
    let mut meili_ok = false;
    let mut errors: Vec<String> = Vec::new();

    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => db_ok = true,
        Err(e) => errors.push(format!("db: {e}")),
    }

    let meili_url = state.cfg.meili_url.trim_end_matches('/');
    match reqwest::Client::new()
        .get(format!("{meili_url}/health"))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => meili_ok = true,
        Ok(res) => errors.push(format!("meili: HTTP {}", res.status())),
        Err(e) => errors.push(format!("meili: {e}")),
    }

    let body = json!({
        "status": if db_ok && meili_ok { "ready" } else { "degraded" },
        "service": "linkdean",
        "checks": {
            "db": db_ok,
            "meilisearch": meili_ok,
        },
        "errors": errors,
    });

    if db_ok && meili_ok {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}
