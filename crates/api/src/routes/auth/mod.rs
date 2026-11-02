mod oidc;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::rate_limit::{client_ip, login_limits, register_limit};
use crate::state::AppState;
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
        .route("/auth/me", get(me))
        .route("/auth/bootstrap-status", get(bootstrap_status))
        .route("/auth/onboarding/complete", post(complete_onboarding))
        .route("/auth/oidc/start", get(oidc::start))
        .route("/auth/oidc/callback", get(oidc::callback))
        .route("/auth/config", get(auth_config))
}

async fn bootstrap_status(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let count = db::count_users(&state.pool).await?;
    let platform = db::get_platform_settings(&state.pool).await?;
    Ok(Json(json!({
        "needs_bootstrap": count == 0,
        "oidc_enabled": state.cfg.oidc_enabled(),
        "registration_open": platform.registration_open,
    })))
}

async fn auth_config(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let platform = db::get_platform_settings(&state.pool).await?;
    Ok(Json(json!({
        "oidc_enabled": state.cfg.oidc_enabled(),
        "app_url": state.cfg.app_url,
        "registration_open": platform.registration_open,
    })))
}

async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.email.trim().is_empty() || req.password.len() < 8 {
        return Err(Error::BadRequest("email required; password min 8 chars".into()).into());
    }
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = register_limit(ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let count = db::count_users(&state.pool).await?;
    let invite_code = req
        .invite_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut must_consume_invite = false;
    if count > 0 {
        let platform = db::get_platform_settings(&state.pool).await?;
        if !platform.registration_open {
            let Some(code) = invite_code else {
                return Err(Error::Forbidden.into());
            };
            db::assert_invite_redeemable(&state.pool, code, &req.email).await?;
            must_consume_invite = true;
        } else if let Some(code) = invite_code {
            // Optional redeem when registration is open.
            db::assert_invite_redeemable(&state.pool, code, &req.email).await?;
            must_consume_invite = true;
        }
    }
    let user = db::create_user(
        &state.pool,
        &req.email,
        Some(&req.password),
        req.display_name.as_deref(),
        None,
        None,
    )
    .await?;
    if must_consume_invite {
        if let Some(code) = invite_code {
            if let Err(e) =
                db::consume_registration_invite(&state.pool, code, &req.email, user.id).await
            {
                tracing::warn!(user_id = %user.id, error = %e, "invite consume failed after register");
                return Err(e.into());
            }
        }
    }
    let share_token = req.share_token.as_deref().map(str::trim);
    if let Some(share_token) = share_token.filter(|t| !t.is_empty()) {
        match db::claim_pending_share_invite(&state.pool, share_token, &req.email, user.id).await {
            Ok(true) => tracing::info!(user_id = %user.id, "claimed share invite"),
            Ok(false) => tracing::info!(user_id = %user.id, "share invite link did not match"),
            Err(e) => tracing::warn!(user_id = %user.id, error = %e, "share invite claim failed"),
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
    let user = state.limiter.settle(&limits, result)?;
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

fn session_response(body: AuthResponse, token: &str, days: i64, secure: bool) -> Response {
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
