use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(accept))
        .routes(routes!(reject))
        .routes(routes!(restore))
        .routes(routes!(claim))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct ClaimRequest {
    token: String,
}

/// Claim an invite link while signed in. The invite must be for this account's address; it
/// then shows up with the other invites to accept.
#[utoipa::path(
    post,
    operation_id = "share_invites_claim",
    path = "/shares/claim",
    tag = "shares",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    get,
    operation_id = "share_invites_list",
    path = "/share-invites",
    tag = "share-invites",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    let invites = db::list_share_invites(&state.pool, user.id).await?;
    Ok(Json(json!({ "invites": invites })))
}

#[utoipa::path(
    post,
    operation_id = "share_invites_accept",
    path = "/share-invites/{id}/accept",
    tag = "share-invites",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn accept(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::accept_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "share_invites_reject",
    path = "/share-invites/{id}/reject",
    tag = "share-invites",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn reject(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::reject_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "share_invites_restore",
    path = "/share-invites/{id}/restore",
    tag = "share-invites",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn restore(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::restore_share_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
