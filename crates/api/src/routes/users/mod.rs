use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::mail::enqueue;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use linkdean_core::avatar::{self, process_avatar};
use linkdean_core::{
    AcceptTerms, ChangePassword, EmailDelivery, Error, NotificationKind, ProfileBookmarksPage,
    ProfileView, ScheduleAccountDeletion, UpdateProfile, UpdateSettings,
};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

/// The image limit in `process_avatar`, plus multipart framing.
const MAX_AVATAR_BODY: usize = 6 * 1024 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(update_me))
        .routes(routes!(change_password))
        .routes(routes!(accept_terms))
        .routes(routes!(schedule_deletion, cancel_deletion))
        .routes(routes!(get_settings, patch_settings))
        .routes(routes!(quota_usage))
        .merge(
            OpenApiRouter::new()
                .routes(routes!(get_avatar, upload_avatar, delete_avatar))
                .layer(DefaultBodyLimit::max(MAX_AVATAR_BODY)),
        )
        .routes(routes!(notification_emails))
        .routes(routes!(set_notification_email))
        .routes(routes!(my_followers))
        .routes(routes!(my_following))
        .routes(routes!(my_blocked))
        .routes(routes!(my_muted))
        .routes(routes!(follow_user, unfollow_user))
        .routes(routes!(follow_status))
        .routes(routes!(block_user, unblock_user))
        .routes(routes!(block_status))
        .routes(routes!(mute_user, unmute_user))
        .routes(routes!(mute_status))
        .routes(routes!(user_profile))
        .routes(routes!(user_bookmarks))
        .routes(routes!(user_followers))
        .routes(routes!(user_following))
        .routes(routes!(user_avatar))
}

fn notification_email_pref(p: db::NotificationEmailPref) -> Value {
    json!({
        "kind": p.kind,
        "label": p.kind.label(),
        "enabled": p.enabled,
        "default": p.kind.email_by_default(),
        "delivery": p.delivery,
        "digest_allowed": p.kind.allows_digest(),
    })
}

/// Which notification kinds the user also gets by email, and how, in settings order.
#[utoipa::path(
    get,
    operation_id = "users_notification_emails",
    path = "/users/me/notification-emails",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn notification_emails(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let prefs = db::get_notification_email_prefs(&state.pool, user.id).await?;
    let prefs: Vec<Value> = prefs.into_iter().map(notification_email_pref).collect();
    Ok(Json(json!({ "prefs": prefs })))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct SetNotificationEmail {
    enabled: bool,
    /// Left out: keep how this kind is delivered now.
    delivery: Option<EmailDelivery>,
}

#[utoipa::path(
    put,
    operation_id = "users_set_notification_email",
    path = "/users/me/notification-emails/{kind}",
    tag = "users",
    params(("kind" = String, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn set_notification_email(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(kind): Path<String>,
    Json(req): Json<SetNotificationEmail>,
) -> ApiResult<Json<Value>> {
    let kind = NotificationKind::parse(&kind).map_err(Error::BadRequest)?;
    let delivery = match req.delivery {
        Some(d) => d,
        None => db::get_notification_email_prefs(&state.pool, user.id)
            .await?
            .into_iter()
            .find(|p| p.kind == kind)
            .map(|p| p.delivery)
            .unwrap_or_default(),
    };
    db::set_notification_email_pref(&state.pool, user.id, kind, req.enabled, delivery).await?;
    Ok(Json(notification_email_pref(db::NotificationEmailPref {
        kind,
        enabled: req.enabled,
        delivery,
    })))
}

#[utoipa::path(
    patch,
    operation_id = "users_update_me",
    path = "/users/me",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn update_me(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<UpdateProfile>,
) -> ApiResult<Json<Value>> {
    let user = db::update_profile(&state.pool, user.id, &body).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

#[utoipa::path(
    post,
    operation_id = "users_change_password",
    path = "/users/me/password",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn change_password(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    session: VaultSession,
    Json(body): Json<ChangePassword>,
) -> ApiResult<Json<Value>> {
    // Every other login session ends; the one making the change stays signed in.
    let (signed_out, revoked_keys) = db::change_password(
        &state.pool,
        user.id,
        &body.current_password,
        &body.new_password,
        session.token_hash(),
        body.revoke_api_keys,
    )
    .await?;
    // A new password locks every vault, in every session.
    state.clear_vault_keys(user.id).await;
    Ok(Json(json!({
        "ok": true,
        "signed_out_sessions": signed_out,
        "revoked_api_keys": revoked_keys,
    })))
}

/// Accepts the current Terms of Service, after they changed since the account last did.
#[utoipa::path(
    post,
    operation_id = "users_accept_terms",
    path = "/users/me/terms",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn accept_terms(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<AcceptTerms>,
) -> ApiResult<Json<Value>> {
    db::accept_terms(&state.pool, user.id, &body.terms_version).await?;
    let user = db::find_user_by_id(&state.pool, user.id)
        .await?
        .ok_or(Error::NotFound)?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

/// Schedules the account for deletion after the grace period. The session stays signed in, so
/// the person can still export their data and cancel.
#[utoipa::path(
    post,
    operation_id = "users_schedule_deletion",
    path = "/users/me/deletion",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn schedule_deletion(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    session: VaultSession,
    Json(body): Json<ScheduleAccountDeletion>,
) -> ApiResult<Json<Value>> {
    let at = db::schedule_account_deletion(
        &state.pool,
        user.id,
        &body.confirm_email,
        body.password,
        session.token_hash(),
    )
    .await?;
    let deletion_date = at.format("%-d %b %Y, %H:%M UTC").to_string();
    if let Err(e) = enqueue(
        &state,
        &user.email,
        "account_deletion_scheduled",
        &json!({ "deletion_date": deletion_date }),
    )
    .await
    {
        tracing::warn!(user_id = %user.id, error = %e, "account deletion email failed");
    }
    Ok(Json(json!({ "deletion_scheduled_at": at })))
}

#[utoipa::path(
    delete,
    operation_id = "users_cancel_deletion",
    path = "/users/me/deletion",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn cancel_deletion(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    db::cancel_account_deletion(&state.pool, user.id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    get,
    operation_id = "users_get_settings",
    path = "/users/me/settings",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    Ok(Json(json!({ "settings": settings })))
}

#[utoipa::path(
    patch,
    operation_id = "users_patch_settings",
    path = "/users/me/settings",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn patch_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<UpdateSettings>,
) -> ApiResult<Json<Value>> {
    let settings = db::update_user_settings(&state.pool, user.id, &body).await?;
    state.vault_keys.set_idle(
        user.id,
        crate::vault_keys::idle_limit(settings.vault_auto_lock_minutes),
    );
    Ok(Json(json!({ "settings": settings })))
}

#[utoipa::path(
    get,
    operation_id = "users_quota_usage",
    path = "/users/me/quota-usage",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn quota_usage(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let usage = crate::quotas::usage_view(&state, user.id).await?;
    Ok(Json(json!(usage)))
}

#[utoipa::path(
    post,
    operation_id = "users_upload_avatar",
    path = "/users/me/avatar",
    tag = "users",
    request_body(content_type = "multipart/form-data", description = "A `file` field with the image."),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn upload_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> ApiResult<Json<Value>> {
    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        let ct = field.content_type().map(|s| s.to_string());
        if name != "avatar" && name != "file" {
            continue;
        }
        if !avatar::accept_content_type(ct.as_deref()) {
            return Err(Error::BadRequest("unsupported content type".into()).into());
        }
        let data = field
            .bytes()
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        bytes = Some(data.to_vec());
        break;
    }
    let raw = bytes.ok_or_else(|| Error::BadRequest("missing avatar file field".into()))?;
    let limits = crate::quotas::limits_for(&state.pool).await?;
    crate::quotas::refuse_upload(
        &state,
        user.id,
        &limits,
        raw.len(),
        crate::quotas::HARD_AVATAR_BODY,
    )
    .await?;
    let webp = tokio::task::spawn_blocking(move || process_avatar(&raw))
        .await
        .map_err(|e| Error::msg(e.to_string()))??;
    let path = state.avatar_path(user.id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    }
    tokio::fs::write(&path, &webp)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let user = db::set_has_avatar(&state.pool, user.id, true).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

#[utoipa::path(
    delete,
    operation_id = "users_delete_avatar",
    path = "/users/me/avatar",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let path = state.avatar_path(user.id);
    if let Err(e) = tokio::fs::remove_file(&path).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(?path, error = %e, "avatar delete failed");
        }
    }
    let user = db::set_has_avatar(&state.pool, user.id, false).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

#[utoipa::path(
    get,
    operation_id = "users_get_avatar",
    path = "/users/me/avatar",
    tag = "users",
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<impl IntoResponse> {
    avatar_response(&state, user.id, user.has_avatar).await
}

// Known issue: `/users/me/avatar` is cached for up to an hour, so after switching accounts the
// previous user's avatar can show. Fix: serve `/users/{id}/avatar` to the web app, or send `no-
// store` for `me`.
async fn avatar_response(
    state: &AppState,
    user_id: Uuid,
    has_avatar: bool,
) -> ApiResult<impl IntoResponse> {
    if !has_avatar {
        return Err(Error::NotFound.into());
    }
    let path = state.avatar_path(user_id);
    let bytes = tokio::fs::read(&path).await.map_err(|_| Error::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    Ok((StatusCode::OK, headers, bytes))
}

#[utoipa::path(
    get,
    operation_id = "users_my_followers",
    path = "/users/me/followers",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn my_followers(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_followers(&state.pool, user.id).await?;
    Ok(Json(json!({ "users": users })))
}

#[utoipa::path(
    get,
    operation_id = "users_my_following",
    path = "/users/me/following",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn my_following(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_following(&state.pool, user.id).await?;
    Ok(Json(json!({ "users": users })))
}

// Known issue: Hidden profiles and blocks can be detected through follower counts and differing
// status codes here, in profile routes and in share routes. Fix: gate them through
// `require_viewable_profile` and return uniform responses.
#[utoipa::path(
    post,
    operation_id = "users_follow_user",
    path = "/users/{id}/follow",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn follow_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::follow_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "follow": relation })))
}

#[utoipa::path(
    delete,
    operation_id = "users_unfollow_user",
    path = "/users/{id}/follow",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unfollow_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::unfollow_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "follow": relation })))
}

#[utoipa::path(
    get,
    operation_id = "users_follow_status",
    path = "/users/{id}/follow-status",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn follow_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::follow_relation(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "follow": relation })))
}

#[utoipa::path(
    get,
    operation_id = "users_my_blocked",
    path = "/users/me/blocked",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn my_blocked(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_blocked(&state.pool, user.id).await?;
    Ok(Json(json!({ "users": users })))
}

#[utoipa::path(
    get,
    operation_id = "users_my_muted",
    path = "/users/me/muted",
    tag = "users",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn my_muted(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let users = db::list_muted(&state.pool, user.id).await?;
    Ok(Json(json!({ "users": users })))
}

#[utoipa::path(
    post,
    operation_id = "users_block_user",
    path = "/users/{id}/block",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn block_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::block_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "block": relation })))
}

#[utoipa::path(
    delete,
    operation_id = "users_unblock_user",
    path = "/users/{id}/block",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unblock_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::unblock_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "block": relation })))
}

#[utoipa::path(
    get,
    operation_id = "users_block_status",
    path = "/users/{id}/block-status",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn block_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::block_relation(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "block": relation })))
}

#[utoipa::path(
    post,
    operation_id = "users_mute_user",
    path = "/users/{id}/mute",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn mute_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::mute_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "mute": relation })))
}

#[utoipa::path(
    delete,
    operation_id = "users_unmute_user",
    path = "/users/{id}/mute",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unmute_user(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::unmute_user(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "mute": relation })))
}

#[utoipa::path(
    get,
    operation_id = "users_mute_status",
    path = "/users/{id}/mute-status",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn mute_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let relation = db::mute_relation(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "mute": relation })))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ProfileBookmarksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[utoipa::path(
    get,
    operation_id = "users_user_profile",
    path = "/users/{id}/profile",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = ProfileView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn user_profile(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProfileView>> {
    Ok(Json(db::get_profile(&state.pool, user.id, id).await?))
}

#[utoipa::path(
    get,
    operation_id = "users_user_bookmarks",
    path = "/users/{id}/bookmarks",
    tag = "users",
    params(("id" = Uuid, Path), ProfileBookmarksQuery),
    responses(
        (status = 200, description = "OK", body = ProfileBookmarksPage),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn user_bookmarks(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<ProfileBookmarksQuery>,
) -> ApiResult<Json<ProfileBookmarksPage>> {
    let page =
        db::profile_visible_bookmarks(&state.pool, user.id, id, query.limit, query.offset).await?;
    Ok(Json(page))
}

#[utoipa::path(
    get,
    operation_id = "users_user_followers",
    path = "/users/{id}/followers",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn user_followers(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let users = db::profile_followers(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "users": users })))
}

#[utoipa::path(
    get,
    operation_id = "users_user_following",
    path = "/users/{id}/following",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn user_following(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let users = db::profile_following(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "users": users })))
}

#[utoipa::path(
    get,
    operation_id = "users_user_avatar",
    path = "/users/{id}/avatar",
    tag = "users",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn user_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let target = db::require_viewable_profile(&state.pool, user.id, id).await?;
    avatar_response(&state, target.id, target.has_avatar).await
}
