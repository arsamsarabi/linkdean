use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::OrganizationInviteInboxItem;
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
}

#[utoipa::path(
    get,
    operation_id = "organization_invites_list",
    path = "/organization-invites",
    tag = "organizations",
    responses(
        (status = 200, description = "OK", body = Vec<OrganizationInviteInboxItem>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<OrganizationInviteInboxItem>>> {
    Ok(Json(
        db::list_organization_invite_inbox(&state.pool, user.id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organization_invites_accept",
    path = "/organization-invites/{id}/accept",
    tag = "organizations",
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
    db::accept_organization_invite(&state.pool, &user, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "organization_invites_reject",
    path = "/organization-invites/{id}/reject",
    tag = "organizations",
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
    db::reject_organization_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "organization_invites_restore",
    path = "/organization-invites/{id}/restore",
    tag = "organizations",
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
    db::restore_organization_invite(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
