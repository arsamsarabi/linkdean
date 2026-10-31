use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{
    CreateDashboard, DashboardSummary, DashboardView, PatchDashboard, PutDashboardLayout,
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
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<DashboardSummary>>> {
    let rows = db::list_dashboards(&state.pool, user.id).await?;
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
