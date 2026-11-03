mod email;

use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::VaultKeyring;
use linkdean_core::{
    ChangeBans, CreateBan, CreateRegistrationInvite, LiftBans, ModerateRequest, UpdateAdminUser,
    UpdatePlatformSettings,
};
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
        .route("/admin/users/{id}/ban", post(ban_user))
        .route(
            "/admin/users/{id}/moderation-actions",
            get(moderation_actions),
        )
        .route("/admin/users/{id}/exposed-content", get(exposed_content))
        .route("/admin/users/{id}/moderate", post(moderate))
        .route("/admin/bans", get(list_bans).patch(change_bans))
        .route("/admin/bans/lift", post(lift_bans))
        .route("/admin/invites", get(list_invites).post(create_invite))
        .route("/admin/invites/{id}", axum::routing::delete(revoke_invite))
        .route("/admin/search/reindex", post(reindex_search))
        .merge(email::router())
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

async fn ban_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateBan>,
) -> ApiResult<Json<Value>> {
    let ban = db::ban_user(&state.pool, admin.id, id, &req).await?;
    state.clear_vault_keys(id).await;
    Ok(Json(json!({ "ban": ban })))
}

async fn list_bans(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let bans = db::list_bans(&state.pool).await?;
    Ok(Json(json!({ "bans": bans })))
}

async fn change_bans(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<ChangeBans>,
) -> ApiResult<Json<Value>> {
    db::change_bans(&state.pool, admin.id, &req).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn lift_bans(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<LiftBans>,
) -> ApiResult<Json<Value>> {
    db::lift_bans(&state.pool, admin.id, &req).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn moderation_actions(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let actions = db::list_moderation_actions(&state.pool, id).await?;
    Ok(Json(json!({ "actions": actions })))
}

async fn exposed_content(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let content = db::exposed_content(&state.pool, id).await?;
    Ok(Json(json!({ "content": content })))
}

async fn moderate(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ModerateRequest>,
) -> ApiResult<Json<Value>> {
    let outcome = db::moderate_content(&state.pool, admin.id, id, &req).await?;
    for bookmark_id in &outcome.trashed_bookmark_ids {
        if let Err(e) = state.search.delete_bookmark(*bookmark_id).await {
            tracing::warn!(%bookmark_id, error = %e, "meili delete failed");
        }
    }
    Ok(Json(json!({ "outcome": outcome })))
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
        let Ok(items) = db::list_bookmarks(
            &state.pool,
            u.id,
            None,
            false,
            &VaultKeyring::default(),
            false,
        )
        .await
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
