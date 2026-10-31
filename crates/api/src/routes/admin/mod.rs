use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::UpdatePlatformSettings;
use linkdean_db::{self as db};
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/platform-settings",
        get(get_platform_settings).patch(patch_platform_settings),
    )
}

async fn get_platform_settings(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let settings = db::get_platform_settings(&state.pool).await?;
    Ok(Json(json!({ "settings": settings })))
}

async fn patch_platform_settings(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Json(req): Json<UpdatePlatformSettings>,
) -> ApiResult<Json<Value>> {
    let settings = if let Some(open) = req.registration_open {
        db::set_registration_open(&state.pool, open).await?
    } else {
        db::get_platform_settings(&state.pool).await?
    };
    Ok(Json(json!({ "settings": settings })))
}
