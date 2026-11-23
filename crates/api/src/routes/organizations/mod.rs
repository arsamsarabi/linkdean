use crate::auth::AuthUser;
use crate::domain_verification;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use linkdean_core::{
    ConfirmOrganizationName, CreateOrganization, CreateOrganizationDomain,
    CreateOrganizationInvite, CreateOrganizationInviteLink, CreateOrganizationInviteLinkResponse,
    CreateTeam, DomainJoinResponse, ListOrganizationAuditQuery, OrganizationAuditEventView,
    OrganizationDomainJoinRequestView, OrganizationDomainSuggestion, OrganizationDomainView,
    OrganizationInviteLinkView, OrganizationInviteView, OrganizationMemberView,
    OrganizationWorkspace, PatchOrganizationDomain, PatchOrganizationInviteLinks,
    PatchOrganizationMember, TeamView,
};
use linkdean_db::{self as db, NewOutboxEmail};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get, leave))
        .routes(routes!(list_members, patch_member, remove_member))
        .routes(routes!(list_teams, create_team))
        .routes(routes!(list_invites, create_invite, revoke_invite))
        .routes(routes!(list_invite_links, create_invite_link))
        .routes(routes!(revoke_invite_link))
        .routes(routes!(patch_invite_links_settings))
        .routes(routes!(list_domains, create_domain))
        .routes(routes!(delete_domain, patch_domain, verify_domain))
        .routes(routes!(list_domain_suggestions))
        .routes(routes!(domain_join))
        .routes(routes!(list_domain_join_requests))
        .routes(routes!(approve_domain_join_request))
        .routes(routes!(deny_domain_join_request))
        .routes(routes!(archive_team))
        .routes(routes!(restore_archived_team))
        .routes(routes!(delete_team))
        .routes(routes!(restore_deleted_team))
        .routes(routes!(soft_delete_org))
        .routes(routes!(restore_org))
        .routes(routes!(list_audit))
        .routes(routes!(export_audit_csv))
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

#[utoipa::path(
    get,
    operation_id = "organizations_list_invites",
    path = "/organizations/{id}/invites",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = Vec<OrganizationInviteView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_invites(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrganizationInviteView>>> {
    Ok(Json(
        db::list_organization_invites(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_create_invite",
    path = "/organizations/{id}/invites",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = OrganizationInviteView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_invite(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateOrganizationInvite>,
) -> ApiResult<Json<OrganizationInviteView>> {
    let (view, _notifiable) = db::create_organization_invite(&state.pool, &user, id, req).await?;
    // Known issue: no branded HTML template yet; plain outbox when mail is configured.
    // Upgrade: share-invite-style template + claim token when A5 UI ships.
    if state.mail.is_some() {
        let inviter = user
            .display_name
            .clone()
            .unwrap_or_else(|| user.email.clone());
        let subject = format!("{inviter} invited you to {}", view.organization_name);
        let text = format!(
            "{inviter} invited you to join {org} on Linkdean as {role}.\n\nSign in (or register with {email}) and open your organization invites to accept.\n",
            org = view.organization_name,
            role = view.role.as_str(),
            email = view.invitee_email,
        );
        let html = format!("<p>{}</p>", text.replace('\n', "<br/>"));
        let _ = db::enqueue_email(
            &state.pool,
            &NewOutboxEmail {
                recipient: view.invitee_email.clone(),
                template: "organization_invite".into(),
                subject,
                html,
                text,
                list_unsubscribe: None,
            },
        )
        .await;
    }
    Ok(Json(view))
}

#[utoipa::path(
    delete,
    operation_id = "organizations_revoke_invite",
    path = "/organizations/{id}/invites/{invite_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("invite_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn revoke_invite(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invite_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::revoke_organization_invite(&state.pool, user.id, id, invite_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_invite_links",
    path = "/organizations/{id}/invite-links",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = Vec<OrganizationInviteLinkView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_invite_links(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrganizationInviteLinkView>>> {
    Ok(Json(
        db::list_organization_invite_links(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_create_invite_link",
    path = "/organizations/{id}/invite-links",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    request_body = CreateOrganizationInviteLink,
    responses(
        (status = 200, description = "OK", body = CreateOrganizationInviteLinkResponse),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_invite_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateOrganizationInviteLink>,
) -> ApiResult<Json<CreateOrganizationInviteLinkResponse>> {
    Ok(Json(
        db::create_organization_invite_link(&state.pool, &user, id, req).await?,
    ))
}

#[utoipa::path(
    delete,
    operation_id = "organizations_revoke_invite_link",
    path = "/organizations/{id}/invite-links/{link_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("link_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn revoke_invite_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, link_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::revoke_organization_invite_link(&state.pool, user.id, id, link_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    patch,
    operation_id = "organizations_patch_invite_links_settings",
    path = "/organizations/{id}/invite-links/settings",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    request_body = PatchOrganizationInviteLinks,
    responses(
        (status = 200, description = "OK", body = OrganizationWorkspace),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn patch_invite_links_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchOrganizationInviteLinks>,
) -> ApiResult<Json<OrganizationWorkspace>> {
    Ok(Json(
        db::set_organization_invite_links_enabled(&state.pool, user.id, id, req).await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_domains",
    path = "/organizations/{id}/domains",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = [OrganizationDomainView]),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_domains(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrganizationDomainView>>> {
    Ok(Json(
        db::list_organization_domains(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_create_domain",
    path = "/organizations/{id}/domains",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    request_body = CreateOrganizationDomain,
    responses(
        (status = 200, description = "OK", body = OrganizationDomainView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_domain(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateOrganizationDomain>,
) -> ApiResult<Json<OrganizationDomainView>> {
    if let Ok(domain) = db::normalize_domain(&req.domain) {
        if domain_verification::is_consumer_mail_domain(&domain)
            || crate::disposable_email::is_disposable_domain(&domain)
        {
            return Err(linkdean_core::Error::BadRequest(
                "That domain can't be verified for an organization.".into(),
            )
            .into());
        }
    }
    Ok(Json(
        db::create_organization_domain(&state.pool, user.id, id, req).await?,
    ))
}

#[utoipa::path(
    delete,
    operation_id = "organizations_delete_domain",
    path = "/organizations/{id}/domains/{domain_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("domain_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_domain(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, domain_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::delete_organization_domain(&state.pool, user.id, id, domain_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    patch,
    operation_id = "organizations_patch_domain",
    path = "/organizations/{id}/domains/{domain_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("domain_id" = Uuid, Path)),
    request_body = PatchOrganizationDomain,
    responses(
        (status = 200, description = "OK", body = OrganizationDomainView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn patch_domain(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, domain_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PatchOrganizationDomain>,
) -> ApiResult<Json<OrganizationDomainView>> {
    Ok(Json(
        db::patch_organization_domain(&state.pool, user.id, id, domain_id, req).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_verify_domain",
    path = "/organizations/{id}/domains/{domain_id}/verify",
    tag = "organizations",
    params(("id" = Uuid, Path), ("domain_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = OrganizationDomainView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn verify_domain(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, domain_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<OrganizationDomainView>> {
    let row = db::get_organization_domain_for_admin(&state.pool, user.id, id, domain_id).await?;
    let records = if row.verified_at.is_some() {
        Vec::new()
    } else {
        let name = domain_verification::challenge_name(&row.domain);
        domain_verification::lookup_txt(&name).await?
    };
    Ok(Json(
        db::verify_organization_domain_with_txt(&state.pool, user.id, id, domain_id, &records)
            .await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_domain_suggestions",
    path = "/organizations/domain-suggestions",
    tag = "organizations",
    responses(
        (status = 200, description = "OK", body = [OrganizationDomainSuggestion]),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_domain_suggestions(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<OrganizationDomainSuggestion>>> {
    Ok(Json(
        db::list_organization_domain_suggestions(&state.pool, user.id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_domain_join",
    path = "/organizations/{id}/domain-join",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DomainJoinResponse),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn domain_join(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DomainJoinResponse>> {
    Ok(Json(
        db::request_or_join_via_domain(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_domain_join_requests",
    path = "/organizations/{id}/domain-join-requests",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = [OrganizationDomainJoinRequestView]),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_domain_join_requests(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrganizationDomainJoinRequestView>>> {
    Ok(Json(
        db::list_organization_domain_join_requests(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_approve_domain_join_request",
    path = "/organizations/{id}/domain-join-requests/{request_id}/approve",
    tag = "organizations",
    params(("id" = Uuid, Path), ("request_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = OrganizationWorkspace),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn approve_domain_join_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, request_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<OrganizationWorkspace>> {
    Ok(Json(
        db::approve_organization_domain_join_request(&state.pool, user.id, id, request_id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_deny_domain_join_request",
    path = "/organizations/{id}/domain-join-requests/{request_id}/deny",
    tag = "organizations",
    params(("id" = Uuid, Path), ("request_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn deny_domain_join_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, request_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::deny_organization_domain_join_request(&state.pool, user.id, id, request_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "organizations_archive_team",
    path = "/organizations/{id}/teams/{team_id}/archive",
    tag = "organizations",
    params(("id" = Uuid, Path), ("team_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = TeamView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn archive_team(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, team_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<TeamView>> {
    Ok(Json(
        db::archive_team(&state.pool, user.id, id, team_id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_restore_archived_team",
    path = "/organizations/{id}/teams/{team_id}/unarchive",
    tag = "organizations",
    params(("id" = Uuid, Path), ("team_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = TeamView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn restore_archived_team(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, team_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<TeamView>> {
    Ok(Json(
        db::restore_archived_team(&state.pool, user.id, id, team_id).await?,
    ))
}

#[utoipa::path(
    delete,
    operation_id = "organizations_delete_team",
    path = "/organizations/{id}/teams/{team_id}",
    tag = "organizations",
    params(("id" = Uuid, Path), ("team_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_team(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, team_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::soft_delete_team(&state.pool, user.id, id, team_id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "organizations_restore_deleted_team",
    path = "/organizations/{id}/teams/{team_id}/restore",
    tag = "organizations",
    params(("id" = Uuid, Path), ("team_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = TeamView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn restore_deleted_team(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, team_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<TeamView>> {
    Ok(Json(
        db::restore_deleted_team(&state.pool, user.id, id, team_id).await?,
    ))
}

#[utoipa::path(
    post,
    operation_id = "organizations_soft_delete",
    path = "/organizations/{id}/delete",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    request_body = ConfirmOrganizationName,
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn soft_delete_org(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ConfirmOrganizationName>,
) -> ApiResult<Json<Value>> {
    let owners = db::soft_delete_organization(&state.pool, user.id, id, &body.confirm_name).await?;
    // Known issue: plain outbox only; branded org-deletion template later.
    if state.mail.is_some() {
        let name = body.confirm_name.trim();
        let subject = format!("{name} is scheduled for deletion");
        let text = format!(
            "The organization \"{name}\" was marked for deletion. Owners can restore it within 30 days from organization settings.\n",
        );
        let html = format!("<p>{}</p>", text.replace('\n', "<br/>"));
        for email in owners {
            let _ = db::enqueue_email(
                &state.pool,
                &NewOutboxEmail {
                    recipient: email,
                    template: "organization_deleted".into(),
                    subject: subject.clone(),
                    html: html.clone(),
                    text: text.clone(),
                    list_unsubscribe: None,
                },
            )
            .await;
        }
    }
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "organizations_restore",
    path = "/organizations/{id}/restore",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = OrganizationWorkspace),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn restore_org(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<OrganizationWorkspace>> {
    Ok(Json(
        db::restore_organization(&state.pool, user.id, id).await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "organizations_list_audit",
    path = "/organizations/{id}/audit",
    tag = "organizations",
    params(
        ("id" = Uuid, Path),
        ("actor_user_id" = Option<Uuid>, Query, description = "Filter by actor"),
        ("action" = Option<String>, Query, description = "Exact action key"),
        ("from" = Option<String>, Query, description = "ISO datetime lower bound"),
        ("to" = Option<String>, Query, description = "ISO datetime upper bound"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
    ),
    responses(
        (status = 200, description = "OK", body = Vec<OrganizationAuditEventView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_audit(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<ListOrganizationAuditQuery>,
) -> ApiResult<Json<Vec<OrganizationAuditEventView>>> {
    Ok(Json(
        db::list_organization_audit_events(&state.pool, user.id, id, &query).await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "organizations_export_audit_csv",
    path = "/organizations/{id}/audit.csv",
    tag = "organizations",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "CSV", content_type = "text/csv"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn export_audit_csv(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let query = ListOrganizationAuditQuery {
        limit: Some(200),
        offset: Some(0),
        ..Default::default()
    };
    let events = db::list_organization_audit_events(&state.pool, user.id, id, &query).await?;
    let mut csv = String::from("created_at,action,actor_user_id,actor_email,payload\n");
    for e in events {
        let payload = e.payload.to_string().replace('"', "\"\"");
        csv.push_str(&format!(
            "{},{},{},{},\"{}\"\n",
            e.created_at.to_rfc3339(),
            e.action,
            e.actor_user_id.map(|u| u.to_string()).unwrap_or_default(),
            e.actor_email.unwrap_or_default().replace(',', " "),
            payload,
        ));
    }
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
        csv,
    ))
}
