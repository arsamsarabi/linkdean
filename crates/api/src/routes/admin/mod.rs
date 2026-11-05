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
        .route("/admin/users/{id}/reset-link", post(create_reset_link))
        .route("/admin/users/{id}/email-verified", post(set_email_verified))
        .route("/admin/users/{id}/email", post(change_email))
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
    let mut settings = db::get_platform_settings(&state.pool).await?;
    if state.cfg.require_verified_email {
        settings.require_verified_email = true;
    }
    Ok(Json(json!({
        "settings": settings,
        "require_verified_email_forced": state.cfg.require_verified_email,
    })))
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
    if let Some(on) = req.require_verified_email {
        if state.cfg.require_verified_email {
            return Err(linkdean_core::Error::BadRequest(
                "REQUIRE_VERIFIED_EMAIL is set by the host; it cannot be changed here".into(),
            )
            .into());
        }
        // Nobody could verify without mail, so every unverified account would be stuck.
        if on && state.mail.is_none() {
            return Err(linkdean_core::Error::BadRequest(
                "turn on outbound email first; nobody can verify without it".into(),
            )
            .into());
        }
        settings = db::set_require_verified_email(&state.pool, on).await?;
    }
    if req.max_bookmarks.is_some()
        || req.max_stored_image_bytes.is_some()
        || req.max_enrich_actions_per_day.is_some()
        || req.max_import_bytes.is_some()
        || req.max_upload_image_bytes.is_some()
    {
        settings = db::set_platform_quotas(
            &state.pool,
            req.max_bookmarks,
            req.max_stored_image_bytes,
            req.max_enrich_actions_per_day,
            req.max_import_bytes,
            req.max_upload_image_bytes,
        )
        .await?;
    }
    if state.cfg.require_verified_email {
        settings.require_verified_email = true;
    }
    Ok(Json(json!({
        "settings": settings,
        "require_verified_email_forced": state.cfg.require_verified_email,
    })))
}

#[derive(serde::Deserialize)]
struct SetEmailVerified {
    verified: bool,
}

/// Mark an account verified (its waiting invites attach) or remove its verification (new
/// invites wait; accepted shares stay).
async fn set_email_verified(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetEmailVerified>,
) -> ApiResult<Json<Value>> {
    db::set_email_verified(&state.pool, id, req.verified).await?;
    Ok(Json(json!({ "ok": true, "email_verified": req.verified })))
}

#[derive(serde::Deserialize)]
struct ChangeEmail {
    email: String,
}

/// Change an account's email at once. The admin vouches for the address, so it counts as
/// verified.
async fn change_email(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeEmail>,
) -> ApiResult<Json<Value>> {
    db::admin_change_email(&state.pool, id, &req.email).await?;
    let email = req.email.trim().to_ascii_lowercase();
    Ok(Json(
        json!({ "ok": true, "email": email, "email_verified": true }),
    ))
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

/// How long an admin-made link works: long enough to hand it over.
const ADMIN_LINK_HOURS: i64 = 24;

/// A reset link to hand over privately; the only way back in when mail is off.
async fn create_reset_link(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let user = db::find_user_by_id(&state.pool, id)
        .await?
        .ok_or(linkdean_core::Error::NotFound)?;
    if user.password_hash.is_none() {
        return Err(linkdean_core::Error::BadRequest(
            "this account signs in with SSO and has no password".into(),
        )
        .into());
    }
    let token = db::create_reset_token(
        &state.pool,
        id,
        chrono::Duration::hours(ADMIN_LINK_HOURS),
        Some(admin.id),
    )
    .await?;
    if let Err(e) =
        crate::mail::enqueue(&state, &user.email, "password_reset_admin", &json!({})).await
    {
        tracing::warn!(error = %e, "admin reset notice email failed");
    }
    let url = crate::routes::auth::reset_url(&state.cfg.app_url, &token);
    Ok(Json(
        json!({ "url": url, "expires_in_hours": ADMIN_LINK_HOURS }),
    ))
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
