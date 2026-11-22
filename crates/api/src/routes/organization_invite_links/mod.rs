use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use linkdean_core::{AcceptOrganizationInviteLink, OrganizationWorkspace};
use linkdean_db::{self as db};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(accept))
}

#[utoipa::path(
    post,
    operation_id = "organization_invite_links_accept",
    path = "/organization-invite-links/accept",
    tag = "organizations",
    request_body = AcceptOrganizationInviteLink,
    responses(
        (status = 200, description = "OK", body = OrganizationWorkspace),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn accept(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<AcceptOrganizationInviteLink>,
) -> ApiResult<Json<OrganizationWorkspace>> {
    Ok(Json(
        db::accept_organization_invite_link(&state.pool, &user, req).await?,
    ))
}
