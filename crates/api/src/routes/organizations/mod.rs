use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::{
    CreateOrganization, CreateTeam, OrganizationMemberView, OrganizationWorkspace,
    PatchOrganizationMember, TeamView,
};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get, leave))
        .routes(routes!(list_members, patch_member, remove_member))
        .routes(routes!(list_teams, create_team))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list",
    path = "/organizations",
    tag = "organizations",
    responses(
        (status = 200, description = "OK", body = Vec<OrganizationWorkspace>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<OrganizationWorkspace>>> {
    Ok(Json(
        db::list_organizations_for_user(&state.pool, user.id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_create",
    path = "/organizations",
    tag = "organizations",
    responses(
        (status = 200, description = "OK", body = OrganizationWorkspace),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateOrganization>,
) -> ApiResult<Json<OrganizationWorkspace>> {
    Ok(Json(
        db::create_organization(&state.pool, &user, req).await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "organizations_get",
    path = "/organizations/{id}",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = OrganizationWorkspace),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<OrganizationWorkspace>> {
    Ok(Json(
        db::get_organization_for_member(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_leave",
    path = "/organizations/{id}/leave",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn leave(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::leave_organization(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_members",
    path = "/organizations/{id}/members",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = Vec<OrganizationMemberView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_members(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrganizationMemberView>>> {
    Ok(Json(
        db::list_organization_members(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    patch,
    operation_id = "organizations_patch_member",
    path = "/organizations/{id}/members/{user_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = OrganizationMemberView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn patch_member(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PatchOrganizationMember>,
) -> ApiResult<Json<OrganizationMemberView>> {
    Ok(Json(
        db::patch_organization_member(&state.pool, user.id, id, user_id, req.role).await?,
    ))
}

#[utoipa::path(
    delete,
    operation_id = "organizations_remove_member",
    path = "/organizations/{id}/members/{user_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn remove_member(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::remove_organization_member(&state.pool, user.id, id, user_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_teams",
    path = "/organizations/{id}/teams",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = Vec<TeamView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_teams(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<TeamView>>> {
    Ok(Json(
        db::list_teams_for_member(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_create_team",
    path = "/organizations/{id}/teams",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = TeamView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_team(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateTeam>,
) -> ApiResult<Json<TeamView>> {
    Ok(Json(db::create_team(&state.pool, user.id, id, req).await?))
}
