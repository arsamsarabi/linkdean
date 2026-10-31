use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use linkdean_core::{
    CreateDashboard, CreateDashboardShare, DashboardShareView, DashboardSummary, DashboardView,
    PatchDashboard, PutDashboardLayout, UpdateDashboardShare,
};
use linkdean_db::{self as db};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboards", get(list).post(create))
        .route("/dashboards/default", get(get_default).put(put_default))
        .route(
            "/dashboards/{id}",
            get(get_one).put(put_one).patch(patch_one).delete(delete_one),
        )
        .route(
            "/dashboards/{id}/shares",
            get(list_shares).post(create_share),
        )
        .route(
            "/dashboards/{id}/shares/{user_id}",
            patch(update_share).delete(delete_share),
        )
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<DashboardSummary>>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let rows =
        db::list_dashboards_filtered(&state.pool, user.id, settings.show_deleted_in_library).await?;
    Ok(Json(rows))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateDashboard>,
) -> ApiResult<Json<DashboardView>> {
    let key = state.vault_key(user.id).await;
    let view = db::create_dashboard(&state.pool, user.id, req, key.as_ref()).await?;
    Ok(Json(view))
}

async fn get_default(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<DashboardView>> {
    let key = state.vault_key(user.id).await;
    let view = db::get_default_dashboard(&state.pool, user.id, key.as_ref()).await?;
    Ok(Json(view))
}

async fn put_default(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<PutDashboardLayout>,
) -> ApiResult<Json<DashboardView>> {
    let key = state.vault_key(user.id).await;
    let view = db::put_default_dashboard(&state.pool, user.id, req, key.as_ref()).await?;
    Ok(Json(view))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<DashboardView>> {
    let key = state.vault_key(user.id).await;
    let view = db::get_dashboard(&state.pool, user.id, id, key.as_ref()).await?;
    Ok(Json(view))
}

async fn put_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PutDashboardLayout>,
) -> ApiResult<Json<DashboardView>> {
    let key = state.vault_key(user.id).await;
    let view = db::put_dashboard(&state.pool, user.id, id, req, key.as_ref()).await?;
    Ok(Json(view))
}

async fn patch_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchDashboard>,
) -> ApiResult<Json<DashboardView>> {
    let key = state.vault_key(user.id).await;
    let view = db::patch_dashboard(&state.pool, user.id, id, req, key.as_ref()).await?;
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
) -> ApiResult<Json<Vec<DashboardShareView>>> {
    let shares = db::list_shares_for_dashboard(&state.pool, user.id, id).await?;
    Ok(Json(shares))
}

async fn create_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateDashboardShare>,
) -> ApiResult<Json<DashboardShareView>> {
    let share = db::create_dashboard_share(&state.pool, user.id, id, req).await?;
    Ok(Json(share))
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
