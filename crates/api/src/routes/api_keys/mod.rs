use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{CreateApiKey, CreateApiKeyResponse};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api-keys", get(list_keys).post(create_key))
        .route("/api-keys/{id}", axum::routing::delete(revoke_key))
}

async fn list_keys(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    db::require_api_keys_allowed(&state.pool, &user).await?;
    let keys = db::list_api_keys(&state.pool, user.id).await?;
    Ok(Json(json!({ "keys": keys })))
}

async fn create_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateApiKey>,
) -> ApiResult<Json<CreateApiKeyResponse>> {
    let created = db::create_api_key(&state.pool, &user, req).await?;
    Ok(Json(created))
}

async fn revoke_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    // Revoke always allowed for own keys (even if gates later flip off)
    db::revoke_api_key(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
