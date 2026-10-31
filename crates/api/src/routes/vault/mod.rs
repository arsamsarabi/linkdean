use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::{Error, VaultSetupRequest, VaultUnlockRequest};
use linkdean_db::{self as db};
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/vault/setup", post(setup))
        .route("/vault/unlock", post(unlock))
        .route("/vault/lock", post(lock))
        .route("/vault/status", get(status))
}

async fn setup(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultSetupRequest>,
) -> ApiResult<Json<Value>> {
    if req.passphrase.len() < 8 {
        return Err(Error::BadRequest("vault passphrase min 8 chars".into()).into());
    }
    db::setup_vault(&state.pool, user.id, &req.passphrase).await?;
    let key = db::unlock_vault_key(&state.pool, user.id, &req.passphrase).await?;
    state.set_vault_key(user.id, key).await;
    Ok(Json(json!({ "ok": true, "unlocked": true })))
}

async fn unlock(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let key = db::unlock_vault_key(&state.pool, user.id, &req.passphrase).await?;
    state.set_vault_key(user.id, key).await;
    Ok(Json(json!({ "ok": true, "unlocked": true })))
}

async fn lock(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    state.clear_vault_key(user.id).await;
    Ok(Json(json!({ "ok": true, "unlocked": false })))
}

async fn status(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({
        "configured": db::user_has_vault(&state.pool, user.id).await?,
        "unlocked": state.vault_key(user.id).await.is_some(),
    })))
}
