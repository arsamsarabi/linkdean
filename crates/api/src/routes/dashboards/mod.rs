use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use linkdean_core::{
    CreateDashboard, CreateDashboardShare, CreateDashboardShareOutcome, DashboardShareView,
    DashboardSharesEnvelope, DashboardSummary, DashboardView, PatchDashboard,
    PendingShareInviteView, PendingShareSubjectType, PutDashboardLayout, UpdateDashboardShare,
    UpdatePendingShareInvite,
};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboards", get(list).post(create))
        .route("/dashboards/default", get(get_default).put(put_default))
        .route(
            "/dashboards/{id}",
            get(get_one)
                .put(put_one)
                .patch(patch_one)
                .delete(delete_one),
        )
        .route(
            "/dashboards/{id}/shares",
            get(list_shares).post(create_share),
        )
        .route(
            "/dashboards/{id}/shares/{user_id}",
            patch(update_share).delete(delete_share),
        )
        .route(
            "/dashboards/{id}/pending-shares/{pending_id}",
            patch(update_pending_share).delete(delete_pending_share),
        )
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<DashboardSummary>>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let rows = db::list_dashboards_filtered(&state.pool, user.id, settings.show_deleted_in_library)
        .await?;
    Ok(Json(rows))
}

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

async fn get_default(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
) -> ApiResult<Json<DashboardView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::get_default_dashboard(&state.pool, user.id, &keyring).await?;
    Ok(Json(view))
}

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

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<DashboardSummary>>> {
    let rows = db::delete_dashboard(&state.pool, user.id, id).await?;
    Ok(Json(rows))
}

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

async fn create_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateDashboardShare>,
) -> ApiResult<Json<CreateDashboardShareOutcome>> {
    let outcome = db::create_dashboard_share(&state.pool, user.id, id, req).await?;
    Ok(Json(outcome))
}

async fn update_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateDashboardShare>,
) -> ApiResult<Json<DashboardShareView>> {
    let share = db::update_dashboard_share(&state.pool, user.id, id, invitee_user_id, req).await?;
    Ok(Json(share))
}

async fn delete_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<()>> {
    db::delete_dashboard_share(&state.pool, user.id, id, invitee_user_id).await?;
    Ok(Json(()))
}

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
