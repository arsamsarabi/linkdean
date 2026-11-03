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
        .route("/shares/claim", post(claim))
}

#[derive(serde::Deserialize)]
struct ClaimRequest {
    token: String,
}

/// Claim an invite link while signed in. The invite must be for this account's address; it
/// then shows up with the other invites to accept.
async fn claim(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<ClaimRequest>,
) -> ApiResult<Json<Value>> {
    if !db::claim_pending_share_invite_signed_in(&state.pool, req.token.trim(), user.id).await? {
        return Err(linkdean_core::Error::BadRequest(
            "this invite link was already used, or was sent to a different email".into(),
        )
        .into());
    }
    Ok(Json(json!({ "ok": true })))
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
