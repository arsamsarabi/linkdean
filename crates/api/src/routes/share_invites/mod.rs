use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/share-invites", get(list))
        .route("/share-invites/{id}/accept", post(accept))
        .route("/share-invites/{id}/reject", post(reject))
        .route("/share-invites/{id}/restore", post(restore))
}

async fn list(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    let invites = db::list_share_invites(&state.pool, user.id).await?;
    Ok(Json(json!({ "invites": invites })))
}

async fn accept(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::accept_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn reject(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::reject_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn restore(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::restore_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
