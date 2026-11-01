use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::{CreateRegistrationInvite, UpdateAdminUser, UpdatePlatformSettings};
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
        .route("/admin/invites", get(list_invites).post(create_invite))
        .route("/admin/invites/{id}", axum::routing::delete(revoke_invite))
        .route("/admin/search/reindex", post(reindex_search))
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

async fn list_invites(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let invites = db::list_registration_invites(&state.pool).await?;
    Ok(Json(json!({ "invites": invites })))
}

async fn create_invite(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<CreateRegistrationInvite>,
) -> ApiResult<Json<Value>> {
    let created = db::create_registration_invite(&state.pool, admin.id, req).await?;
    Ok(Json(json!({
        "invite": created.invite,
        "code": created.code,
    })))
}

async fn revoke_invite(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::revoke_registration_invite(&state.pool, id).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Clear Meili bookmarks index and re-upsert all non-vault, non-deleted bookmarks.
async fn reindex_search(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    if let Err(e) = state.search.ensure_index().await {
        tracing::warn!("meili ensure_index before reindex: {e}");
    }
    if let Err(e) = state.search.clear_index().await {
        return Err(linkdean_core::Error::msg(format!("meili clear failed: {e}")).into());
    }

    let users = db::list_admin_users(&state.pool).await?;
    let mut upserted: u64 = 0;
    let mut failed: u64 = 0;

    for u in users {
        let Ok(items) =
            db::list_bookmarks(&state.pool, u.id, None, false, None, false).await
        else {
            continue;
        };
        for v in items {
            // Shared rows appear under multiple invitees; upsert by id is idempotent.
            // Prefer owner-owned docs: skip when listing through a non-owner (shared).
            if v.owner_user_id != u.id {
                continue;
            }
            match state.search.upsert_bookmark(&v).await {
                Ok(()) => upserted += 1,
                Err(e) => {
                    failed += 1;
                    tracing::warn!(bookmark_id = %v.id, error = %e, "meili reindex upsert failed");
                }
            }
        }
    }

    Ok(Json(json!({
        "ok": true,
        "upserted": upserted,
        "failed": failed,
    })))
}
