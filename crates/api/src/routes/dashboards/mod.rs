use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{DashboardView, PutDashboardLayout};
use linkdean_db::{self as db};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/dashboards/default",
        get(get_default).put(put_default),
    )
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
