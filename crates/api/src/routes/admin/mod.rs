use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{UpdateAdminUser, UpdatePlatformSettings};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/platform-settings",
            get(get_platform_settings).patch(patch_platform_settings),
        )
        .route("/admin/users", get(list_users))
        .route("/admin/users/{id}", axum::routing::patch(patch_user))
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
    let mut settings = db::get_platform_settings(&state.pool).await?;
    if let Some(open) = req.registration_open {
        settings = db::set_registration_open(&state.pool, open).await?;
    }
    if let Some(enabled) = req.api_keys_enabled {
        settings = db::set_api_keys_enabled(&state.pool, enabled).await?;
    }
    Ok(Json(json!({ "settings": settings })))
}

async fn list_users(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_admin_users(&state.pool).await?;
    Ok(Json(json!({ "users": users })))
}

async fn patch_user(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAdminUser>,
) -> ApiResult<Json<Value>> {
    let user = db::update_admin_user(&state.pool, id, req).await?;
    Ok(Json(json!({ "user": user })))
}
