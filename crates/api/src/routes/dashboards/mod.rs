use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::notification_mail::{self, InviteEmail};
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::{
    CreateDashboard, CreateDashboardShare, CreateDashboardShareOutcome, DashboardShareView,
    DashboardSharesEnvelope, DashboardSummary, DashboardView, PatchDashboard,
    PendingShareInviteView, PendingShareSubjectType, PutDashboardLayout, UpdateDashboardShare,
    UpdatePendingShareInvite,
};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(get_default, put_default))
        .routes(routes!(get_one, put_one, patch_one, delete_one))
        .routes(routes!(list_shares, create_share))
        .routes(routes!(update_share, delete_share))
        .routes(routes!(update_pending_share, delete_pending_share))
}

#[utoipa::path(
    get,
    operation_id = "dashboards_list",
    path = "/dashboards",
    tag = "dashboards",
    responses(
        (status = 200, description = "OK", body = Vec<DashboardSummary>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<DashboardSummary>>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let rows = db::list_dashboards_filtered(&state.pool, user.id, settings.show_deleted_in_library)
        .await?;
    Ok(Json(rows))
}

#[utoipa::path(
    post,
    operation_id = "dashboards_create",
    path = "/dashboards",
    tag = "dashboards",
    responses(
        (status = 200, description = "OK", body = DashboardView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<CreateDashboard>,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::create_dashboard(&state.pool, user.id, req, &keyring).await?;
    Ok(Json(view))
}

#[utoipa::path(
    get,
    operation_id = "dashboards_get_default",
    path = "/dashboards/default",
    tag = "dashboards",
    responses(
        (status = 200, description = "OK", body = DashboardView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get_default(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::get_default_dashboard(&state.pool, user.id, &keyring).await?;
    Ok(Json(view))
}

#[utoipa::path(
    put,
    operation_id = "dashboards_put_default",
    path = "/dashboards/default",
    tag = "dashboards",
    responses(
        (status = 200, description = "OK", body = DashboardView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn put_default(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<PutDashboardLayout>,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::put_default_dashboard(&state.pool, user.id, req, &keyring).await?;
    Ok(Json(view))
}

#[utoipa::path(
    get,
    operation_id = "dashboards_get_one",
    path = "/dashboards/{id}",
    tag = "dashboards",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DashboardView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::get_dashboard(&state.pool, user.id, id, &keyring).await?;
    Ok(Json(view))
}

#[utoipa::path(
    put,
    operation_id = "dashboards_put_one",
    path = "/dashboards/{id}",
    tag = "dashboards",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DashboardView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn put_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<PutDashboardLayout>,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::put_dashboard(&state.pool, user.id, id, req, &keyring).await?;
    Ok(Json(view))
}

#[utoipa::path(
    patch,
    operation_id = "dashboards_patch_one",
    path = "/dashboards/{id}",
    tag = "dashboards",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DashboardView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn patch_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchDashboard>,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::patch_dashboard(&state.pool, user.id, id, req, &keyring).await?;
    Ok(Json(view))
}

#[utoipa::path(
    delete,
    operation_id = "dashboards_delete_one",
    path = "/dashboards/{id}",
    tag = "dashboards",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = Vec<DashboardSummary>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<DashboardSummary>>> {
    let rows = db::delete_dashboard(&state.pool, user.id, id).await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    operation_id = "dashboards_list_shares",
    path = "/dashboards/{id}/shares",
    tag = "dashboards",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DashboardSharesEnvelope),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_shares(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DashboardSharesEnvelope>> {
    let shares = db::list_shares_for_dashboard(&state.pool, user.id, id).await?;
    let pending =
        db::list_pending_for_subject(&state.pool, user.id, PendingShareSubjectType::Dashboard, id)
            .await?;
    Ok(Json(DashboardSharesEnvelope { shares, pending }))
}

#[utoipa::path(
    post,
    operation_id = "dashboards_create_share",
    path = "/dashboards/{id}/shares",
    tag = "dashboards",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateDashboardShare>,
) -> ApiResult<Json<Value>> {
    let since = notification_mail::invite_clock(&state).await;
    let outcome = db::create_dashboard_share(&state.pool, user.id, id, req).await?;
    let email = match &outcome {
        CreateDashboardShareOutcome::Pending { invite } => {
            notification_mail::invite_email(&state, &user, invite, since).await
        }
        CreateDashboardShareOutcome::Active { .. } => InviteEmail::default(),
    };
    Ok(Json(notification_mail::with_invite_email(&outcome, email)))
}

#[utoipa::path(
    patch,
    operation_id = "dashboards_update_share",
    path = "/dashboards/{id}/shares/{user_id}",
    tag = "dashboards",
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = DashboardShareView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn update_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateDashboardShare>,
) -> ApiResult<Json<DashboardShareView>> {
    let share = db::update_dashboard_share(&state.pool, user.id, id, invitee_user_id, req).await?;
    Ok(Json(share))
}

#[utoipa::path(
    delete,
    operation_id = "dashboards_delete_share",
    path = "/dashboards/{id}/shares/{user_id}",
    tag = "dashboards",
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = ()),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<()>> {
    db::delete_dashboard_share(&state.pool, user.id, id, invitee_user_id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    patch,
    operation_id = "dashboards_update_pending_share",
    path = "/dashboards/{id}/pending-shares/{pending_id}",
    tag = "dashboards",
    params(("id" = Uuid, Path), ("pending_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = PendingShareInviteView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn update_pending_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, pending_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePendingShareInvite>,
) -> ApiResult<Json<PendingShareInviteView>> {
    let invite = db::update_pending_share_invite(
        &state.pool,
        user.id,
        PendingShareSubjectType::Dashboard,
        id,
        pending_id,
        req,
    )
    .await?;
    Ok(Json(invite))
}

#[utoipa::path(
    delete,
    operation_id = "dashboards_delete_pending_share",
    path = "/dashboards/{id}/pending-shares/{pending_id}",
    tag = "dashboards",
    params(("id" = Uuid, Path), ("pending_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_pending_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, pending_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::revoke_pending_share_invite(
        &state.pool,
        user.id,
        PendingShareSubjectType::Dashboard,
        id,
        pending_id,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}
