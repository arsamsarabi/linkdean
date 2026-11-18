mod email;
mod reports;

use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::VaultKeyring;
use linkdean_core::{
    ChangeBans, CreateBan, CreateRegistrationInvite, LiftBans, ModerateRequest, UpdateAdminUser,
    UpdatePlatformSettings,
};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_platform_settings, patch_platform_settings))
        .routes(routes!(list_users))
        .routes(routes!(patch_user))
        .routes(routes!(ban_user))
        .routes(routes!(create_reset_link))
        .routes(routes!(set_email_verified))
        .routes(routes!(change_email))
        .routes(routes!(moderation_actions))
        .routes(routes!(exposed_content))
        .routes(routes!(moderate))
        .routes(routes!(list_bans, change_bans))
        .routes(routes!(lift_bans))
        .routes(routes!(list_invites, create_invite))
        .routes(routes!(revoke_invite))
        .routes(routes!(reindex_search))
        .routes(routes!(screenshot_retry_status, retry_screenshots))
        .merge(email::router())
        .merge(reports::router())
}

#[utoipa::path(
    get,
    operation_id = "admin_get_platform_settings",
    path = "/admin/platform-settings",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    patch,
    operation_id = "admin_patch_platform_settings",
    path = "/admin/platform-settings",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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
    if req.screenshot_retry_cooldown_hours.is_some() || req.screenshot_auto_retry_days.is_some() {
        if req
            .screenshot_retry_cooldown_hours
            .is_some_and(|h| !(1..=720).contains(&h))
        {
            return Err(linkdean_core::Error::BadRequest(
                "the retry cooldown must be between 1 and 720 hours".into(),
            )
            .into());
        }
        if req
            .screenshot_auto_retry_days
            .is_some_and(|d| !(1..=90).contains(&d))
        {
            return Err(linkdean_core::Error::BadRequest(
                "the automatic retry window must be between 1 and 90 days".into(),
            )
            .into());
        }
        settings = db::set_screenshot_retry_settings(
            &state.pool,
            req.screenshot_retry_cooldown_hours,
            req.screenshot_auto_retry_days,
        )
        .await?;
    }
    if req.archives_enabled.is_some()
        || req.max_archive_bytes.is_some()
        || req.max_archive_item_bytes.is_some()
    {
        if req.max_archive_bytes.flatten().is_some_and(|n| n < 0) {
            return Err(linkdean_core::Error::BadRequest(
                "archive storage cannot be negative".into(),
            )
            .into());
        }
        if req
            .max_archive_item_bytes
            .is_some_and(|n| !(1_048_576..=1_073_741_824).contains(&n))
        {
            return Err(linkdean_core::Error::BadRequest(
                "the largest archived copy must be between 1 MB and 1 GB".into(),
            )
            .into());
        }
        settings = db::set_archive_settings(
            &state.pool,
            req.archives_enabled,
            req.max_archive_bytes,
            req.max_archive_item_bytes,
        )
        .await?;
    }
    if let Some(email) = req.abuse_contact_email {
        let email = linkdean_core::clean_abuse_contact_email(email.as_deref())?;
        settings = db::set_abuse_contact_email(&state.pool, email.as_deref()).await?;
    }
    if state.cfg.require_verified_email {
        settings.require_verified_email = true;
    }
    Ok(Json(json!({
        "settings": settings,
        "require_verified_email_forced": state.cfg.require_verified_email,
    })))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct SetEmailVerified {
    verified: bool,
}

/// Mark an account verified (its waiting invites attach) or remove its verification (new
/// invites wait; accepted shares stay).
#[utoipa::path(
    post,
    operation_id = "admin_set_email_verified",
    path = "/admin/users/{id}/email-verified",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn set_email_verified(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetEmailVerified>,
) -> ApiResult<Json<Value>> {
    db::set_email_verified(&state.pool, id, req.verified).await?;
    Ok(Json(json!({ "ok": true, "email_verified": req.verified })))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct ChangeEmail {
    email: String,
}

/// Change an account's email at once. The admin vouches for the address, so it counts as
/// verified.
#[utoipa::path(
    post,
    operation_id = "admin_change_email",
    path = "/admin/users/{id}/email",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    get,
    operation_id = "admin_list_users",
    path = "/admin/users",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn list_users(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_admin_users(&state.pool).await?;
    Ok(Json(json!({ "users": users })))
}

#[utoipa::path(
    patch,
    operation_id = "admin_patch_user",
    path = "/admin/users/{id}",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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
#[utoipa::path(
    post,
    operation_id = "admin_create_reset_link",
    path = "/admin/users/{id}/reset-link",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    post,
    operation_id = "admin_ban_user",
    path = "/admin/users/{id}/ban",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    get,
    operation_id = "admin_list_bans",
    path = "/admin/bans",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn list_bans(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let bans = db::list_bans(&state.pool).await?;
    Ok(Json(json!({ "bans": bans })))
}

#[utoipa::path(
    patch,
    operation_id = "admin_change_bans",
    path = "/admin/bans",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn change_bans(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<ChangeBans>,
) -> ApiResult<Json<Value>> {
    db::change_bans(&state.pool, admin.id, &req).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "admin_lift_bans",
    path = "/admin/bans/lift",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn lift_bans(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<LiftBans>,
) -> ApiResult<Json<Value>> {
    db::lift_bans(&state.pool, admin.id, &req).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    operation_id = "admin_moderation_actions",
    path = "/admin/users/{id}/moderation-actions",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn moderation_actions(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let actions = db::list_moderation_actions(&state.pool, id).await?;
    Ok(Json(json!({ "actions": actions })))
}

#[utoipa::path(
    get,
    operation_id = "admin_exposed_content",
    path = "/admin/users/{id}/exposed-content",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn exposed_content(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let content = db::exposed_content(&state.pool, id).await?;
    Ok(Json(json!({ "content": content })))
}

#[utoipa::path(
    post,
    operation_id = "admin_moderate",
    path = "/admin/users/{id}/moderate",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    get,
    operation_id = "admin_list_invites",
    path = "/admin/invites",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn list_invites(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let invites = db::list_registration_invites(&state.pool).await?;
    Ok(Json(json!({ "invites": invites })))
}

#[utoipa::path(
    post,
    operation_id = "admin_create_invite",
    path = "/admin/invites",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    delete,
    operation_id = "admin_revoke_invite",
    path = "/admin/invites/{id}",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn revoke_invite(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::revoke_registration_invite(&state.pool, id).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Clear Meili bookmarks index and re-upsert all non-vault, non-deleted bookmarks.
#[utoipa::path(
    post,
    operation_id = "admin_reindex_search",
    path = "/admin/search/reindex",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
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
            match crate::search_index::upsert(&state, &v).await {
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

/// What the retry button needs: how many failed screenshots a retry would take, and when it
/// is next allowed.
#[utoipa::path(
    get,
    operation_id = "admin_screenshot_retry_status",
    path = "/admin/screenshots/retry",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn screenshot_retry_status(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let settings = db::get_platform_settings(&state.pool).await?;
    retry_status_json(&state, &settings).await
}

async fn retry_status_json(
    state: &AppState,
    settings: &linkdean_core::PlatformSettings,
) -> ApiResult<Json<Value>> {
    let cooldown = settings.screenshot_retry_cooldown_hours;
    let failed = db::count_failed_previews(&state.pool, cooldown).await?;
    let next_available_at = settings
        .screenshot_retry_last_run_at
        .map(|t| t + chrono::Duration::hours(i64::from(cooldown)))
        .filter(|t| *t > chrono::Utc::now());
    Ok(Json(json!({
        "failed": failed,
        "cooldown_hours": cooldown,
        "last_run_at": settings.screenshot_retry_last_run_at,
        "next_available_at": next_available_at,
        "running": linkdean_worker::backfill_running(),
        "browser_configured": state.enrich_opts().browser_ws_url.is_some(),
    })))
}

/// Queue failed screenshots for another try, at most once per cooldown. Only failures older
/// than the cooldown are taken, so a page that just failed is not tried again at once.
#[utoipa::path(
    post,
    operation_id = "admin_retry_screenshots",
    path = "/admin/screenshots/retry",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn retry_screenshots(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let opts = state.enrich_opts();
    if opts.browser_ws_url.is_none() {
        return Err(linkdean_core::Error::BadRequest(
            "screenshots are off on this server (BROWSER_WS_URL is not set)".into(),
        )
        .into());
    }
    let Some(guard) = linkdean_worker::try_begin_backfill() else {
        return Err(linkdean_core::Error::Conflict(
            "screenshots are already being taken; try again when they finish".into(),
        )
        .into());
    };
    if db::claim_screenshot_retry(&state.pool).await?.is_none() {
        return Err(linkdean_core::Error::Conflict(
            "failed screenshots were retried recently; wait for the cooldown".into(),
        )
        .into());
    }
    let settings = db::get_platform_settings(&state.pool).await?;
    let rows =
        db::list_failed_previews(&state.pool, settings.screenshot_retry_cooldown_hours).await?;
    let queued = rows.len();
    tokio::spawn(linkdean_worker::retry_failed_previews(
        guard,
        state.pool.clone(),
        state.search.clone(),
        opts,
        rows,
    ));
    let Json(mut status) = retry_status_json(&state, &settings).await?;
    status["queued"] = json!(queued);
    Ok(Json(status))
}
