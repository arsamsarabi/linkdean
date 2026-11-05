mod email_verification;
mod oidc;
mod password_reset;

pub(crate) use password_reset::reset_url;

use crate::auth::AuthUser;
use crate::disposable_email;
use crate::error::ApiResult;
use crate::rate_limit::{client_ip, login_limits, register_limit};
use crate::state::AppState;
use crate::turnstile;
use crate::vault_keys::VaultSession;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::{AuthResponse, Error, LoginRequest, RegisterRequest};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use std::net::SocketAddr;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/password/forgot", post(password_reset::forgot))
        .route("/auth/password/reset", post(password_reset::reset))
        .route("/auth/email/resend", post(email_verification::resend))
        .route("/auth/email/verify", post(email_verification::verify))
        .route("/auth/email/change", post(email_verification::change))
        .route(
            "/auth/email/confirm-change",
            post(email_verification::confirm_change),
        )
        .route("/auth/me", get(me))
        .route("/auth/bootstrap-status", get(bootstrap_status))
        .route("/auth/onboarding/complete", post(complete_onboarding))
        .route("/auth/oidc/start", get(oidc::start))
        .route("/auth/oidc/callback", get(oidc::callback))
        .route("/auth/oidc/pending-link", get(oidc::pending_link))
        .route("/auth/oidc/disconnect", post(oidc::disconnect))
        .route("/auth/config", get(auth_config))
}

async fn bootstrap_status(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let count = db::count_users(&state.pool).await?;
    let platform = db::get_platform_settings(&state.pool).await?;
    Ok(Json(json!({
        "needs_bootstrap": count == 0,
        "bootstrap_token_required": count == 0 && state.cfg.bootstrap_token.is_some(),
        "oidc_enabled": state.cfg.oidc_enabled(),
        "registration_open": platform.registration_open,
        "mail_enabled": state.mail.is_some(),
        "turnstile_site_key": state.cfg.turnstile_site_key,
    })))
}

async fn auth_config(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let platform = db::get_platform_settings(&state.pool).await?;
    Ok(Json(json!({
        "oidc_enabled": state.cfg.oidc_enabled(),
        "app_url": state.cfg.app_url,
        "registration_open": platform.registration_open,
        "mail_enabled": state.mail.is_some(),
        "turnstile_site_key": state.cfg.turnstile_site_key,
    })))
}

/// Local sign-up. The first account becomes admin and needs `BOOTSTRAP_TOKEN` when one is set;
/// later ones need open registration or an invite. `create_user_gated` checks both and claims
/// the invite under one lock and transaction.
async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    let min = linkdean_core::crypto::MIN_SECRET_LEN;
    if req.email.trim().is_empty() || req.password.len() < min {
        return Err(Error::BadRequest(format!("email required; password min {min} chars")).into());
    }
    disposable_email::refuse_if_disposable(&req.email, state.cfg.block_disposable_emails)?;
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    if state.cfg.turnstile_enabled() {
        let secret = state
            .cfg
            .turnstile_secret_key
            .as_deref()
            .expect("turnstile_enabled implies secret");
        turnstile::verify(
            &reqwest::Client::new(),
            turnstile::SITEVERIFY_URL,
            secret,
            req.turnstile_token.as_deref().unwrap_or(""),
            Some(ip),
        )
        .await?;
    }
    let limits = register_limit(ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let invite_code = req
        .invite_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut user = db::create_user_gated(
        &state.pool,
        &req.email,
        Some(&req.password),
        req.display_name.as_deref(),
        None,
        None,
        db::SignupGate {
            enforce: true,
            invite_code,
            bootstrap_token_ok: state.cfg.bootstrap_token_ok(req.bootstrap_token.as_deref()),
        },
    )
    .await?;
    let share_token = req.share_token.as_deref().map(str::trim);
    if let Some(share_token) = share_token.filter(|t| !t.is_empty()) {
        match db::claim_pending_share_invite(&state.pool, share_token, &req.email, user.id).await {
            Ok(true) => {
                tracing::info!(user_id = %user.id, "claimed share invite");
                // The link from an invite email also verifies the address.
                if let Ok(Some(fresh)) = db::find_user_by_id(&state.pool, user.id).await {
                    user = fresh;
                }
            }
            Ok(false) => tracing::info!(user_id = %user.id, "share invite link did not match"),
            Err(e) => tracing::warn!(user_id = %user.id, error = %e, "share invite claim failed"),
        }
    }
    // Best effort: the account works unverified, and Resend sends another link.
    if user.email_verified_at.is_none() {
        if let Err(e) = email_verification::send_verify_email(&state, user.id, &user.email).await {
            tracing::warn!(user_id = %user.id, error = %e, "verification email failed");
        }
    }
    let token = db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(session_response(
        AuthResponse {
            user: public,
            token: token.clone(),
        },
        &token,
        state.cfg.session_days,
        state.cfg.cookie_secure(),
    ))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = login_limits(ip, &req.email);
    state.limiter.check(&limits)?;
    let result = db::authenticate_local(&state.pool, &req.email, &req.password).await;
    let mut user = state.limiter.settle(&limits, result)?;
    // A pending single sign-on link from this browser attaches now, if this account holds the
    // provider's address. Linking never fails the sign-in.
    let link_token = oidc::read_link_cookie(&headers);
    let mut sso_connected = false;
    if let Some(link_token) = &link_token {
        match db::consume_pending_oidc_link(&state.pool, link_token, user.id).await {
            Ok(Some(link)) => {
                sso_connected = true;
                tracing::info!(user_id = %user.id, "single sign-on linked at sign-in");
                oidc::notify_sso_connected(&state, user.id, &user.email, &link.issuer).await;
                if let Ok(Some(fresh)) = db::find_user_by_id(&state.pool, user.id).await {
                    user = fresh;
                }
            }
            Ok(None) => tracing::info!(user_id = %user.id, "pending single sign-on link not used"),
            Err(e) => {
                tracing::warn!(user_id = %user.id, error = %e, "pending single sign-on link refused")
            }
        }
    }
    let token = db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
    let public = db::to_public(&state.pool, &user).await?;
    let mut body = serde_json::to_value(AuthResponse {
        user: public,
        token: token.clone(),
    })
    .map_err(|e| Error::msg(e.to_string()))?;
    if sso_connected {
        body["sso_connected"] = json!(true);
    }
    let secure = state.cfg.cookie_secure();
    let mut res = session_response(body, &token, state.cfg.session_days, secure);
    if link_token.is_some() {
        // Used or not, the link is done with once the browser signs in.
        match HeaderValue::from_str(&oidc::link_cookie("", 0, secure)) {
            Ok(v) => {
                res.headers_mut().append(axum::http::header::SET_COOKIE, v);
            }
            Err(_) => tracing::error!("failed to expire the pending link cookie"),
        }
    }
    Ok(res)
}

async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Response> {
    if let Some(token) = bearer_or_cookie(&headers) {
        state
            .vault_keys
            .clear_session(&VaultSession::from_token(&token));
        db::delete_session(&state.pool, &token).await?;
    }
    // Expire the cookie too, so a cookie-only (OIDC) browser is signed out.
    let mut res = Json(json!({ "ok": true })).into_response();
    let cookie = session_cookie("", 0, state.cfg.cookie_secure());
    res.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| Error::msg(e.to_string()))?,
    );
    Ok(res)
}

async fn me(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
) -> ApiResult<Json<Value>> {
    let public = db::to_public(&state.pool, &user).await?;
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let vault_unlocked = !state
        .vault_keyring(&vault_session, user.id)
        .await
        .is_empty();
    let platform = db::get_platform_settings(&state.pool).await?;
    let api_keys_allowed = platform.api_keys_enabled && user.api_keys_enabled;
    Ok(Json(json!({
        "user": public,
        "settings": settings,
        "vault_unlocked": vault_unlocked,
        "api_keys_allowed": api_keys_allowed,
        "require_verified_email": (platform.require_verified_email
            || state.cfg.require_verified_email)
            && state.mail.is_some(),
    })))
}

async fn complete_onboarding(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let user = db::complete_onboarding(&state.pool, user.id).await?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "user": public })))
}

fn session_response(body: impl serde::Serialize, token: &str, days: i64, secure: bool) -> Response {
    let cookie = session_cookie(token, days, secure);
    let mut res = Json(body).into_response();
    if let Ok(v) = HeaderValue::from_str(&cookie) {
        res.headers_mut().insert(axum::http::header::SET_COOKIE, v);
    } else {
        tracing::error!("failed to set session cookie header");
    }
    res
}

pub(super) fn session_cookie(token: &str, days: i64, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "linkdean_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{secure_flag}",
        days * 86400
    )
}

fn bearer_or_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(h) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = h.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return Some(rest.to_string());
            }
        }
    }
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|c| {
            c.split(';').find_map(|part| {
                part.trim()
                    .strip_prefix("linkdean_session=")
                    .map(|v| v.to_string())
            })
        })
}
