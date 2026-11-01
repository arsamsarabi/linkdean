use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::{AuthResponse, Error, LoginRequest, RegisterRequest};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/bootstrap-status", get(bootstrap_status))
        .route("/auth/onboarding/complete", post(complete_onboarding))
        .route("/auth/oidc/start", get(oidc_start))
        .route("/auth/oidc/callback", get(oidc_callback))
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
    Json(req): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.email.trim().is_empty() || req.password.len() < 8 {
        return Err(Error::BadRequest("email required; password min 8 chars".into()).into());
    }
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
    match db::link_pending_share_invites(&state.pool, &req.email, user.id).await {
        Ok(n) if n > 0 => {
            tracing::info!(user_id = %user.id, linked = n, "linked pending share invites")
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(user_id = %user.id, error = %e, "pending share link failed"),
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
    Json(req): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let user = db::authenticate_local(&state.pool, &req.email, &req.password).await?;
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
) -> ApiResult<Json<Value>> {
    if let Some(token) = bearer_or_cookie(&headers) {
        if let Ok(Some(user)) = db::user_from_token(&state.pool, &token).await {
            state.clear_vault_keys(user.id).await;
        }
        db::delete_session(&state.pool, &token).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

async fn me(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    let public = db::to_public(&state.pool, &user).await?;
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let vault_unlocked = !state.vault_keyring(user.id).await.is_empty();
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

async fn oidc_start(State(state): State<AppState>) -> ApiResult<Redirect> {
    if !state.cfg.oidc_enabled() {
        return Err(Error::BadRequest("OIDC not configured".into()).into());
    }
    let issuer = state
        .cfg
        .oidc_issuer
        .as_deref()
        .ok_or_else(|| Error::BadRequest("OIDC not configured".into()))?;
    let client_id = state
        .cfg
        .oidc_client_id
        .as_deref()
        .ok_or_else(|| Error::BadRequest("OIDC not configured".into()))?;
    let redirect = format!(
        "{}/api/auth/oidc/callback",
        state.cfg.app_url.trim_end_matches('/')
    );
    // Authentik-style authorize path when issuer is .../application/o/<slug>/
    let authorize = format!(
        "{}/application/o/authorize/?client_id={}&response_type=code&scope=openid%20profile%20email&redirect_uri={}",
        issuer
            .trim_end_matches('/')
            .trim_end_matches("/.well-known/openid-configuration"),
        urlencoding_encode(client_id),
        urlencoding_encode(&redirect),
    );
    Ok(Redirect::temporary(&authorize))
}

#[derive(Deserialize)]
struct OidcCb {
    code: Option<String>,
    error: Option<String>,
}

async fn oidc_callback(
    State(state): State<AppState>,
    Query(q): Query<OidcCb>,
) -> ApiResult<Response> {
    if let Some(err) = q.error {
        return Err(Error::BadRequest(err).into());
    }
    let code = q.code.ok_or(Error::BadRequest("missing code".into()))?;
    if !state.cfg.oidc_enabled() {
        return Err(Error::BadRequest("OIDC not configured".into()).into());
    }

    let issuer = state
        .cfg
        .oidc_issuer
        .clone()
        .ok_or_else(|| Error::BadRequest("OIDC not configured".into()))?;
    let client_id = state
        .cfg
        .oidc_client_id
        .clone()
        .ok_or_else(|| Error::BadRequest("OIDC not configured".into()))?;
    let client_secret = state
        .cfg
        .oidc_client_secret
        .clone()
        .ok_or_else(|| Error::BadRequest("OIDC not configured".into()))?;
    let redirect = format!(
        "{}/api/auth/oidc/callback",
        state.cfg.app_url.trim_end_matches('/')
    );

    let token_url = format!("{}/token/", issuer.trim_end_matches('/'));
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| Error::msg(e.to_string()))?;
    let resp = http
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::error!(%body, "OIDC token exchange failed");
        return Err(Error::BadRequest("OIDC token exchange failed".into()).into());
    }
    let token_json: Value = resp.json().await.map_err(|e| Error::msg(e.to_string()))?;
    let access = token_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or(Error::BadRequest("no access_token".into()))?;

    let userinfo_url = format!("{}/userinfo/", issuer.trim_end_matches('/'));
    let info: Value = http
        .get(&userinfo_url)
        .bearer_auth(access)
        .send()
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .json()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let sub = info
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or(Error::BadRequest("no sub".into()))?;
    let email = info.get("email").and_then(|v| v.as_str()).unwrap_or(sub);
    let name = info
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Never auto-login by email alone (account takeover). Match OIDC subject or create.
    let user = match db::find_user_by_oidc(&state.pool, &issuer, sub).await? {
        Some(u) => u,
        None => {
            if db::find_user_by_email(&state.pool, email).await?.is_some() {
                return Err(Error::Conflict(
                    "email already registered; sign in with password (OIDC auto-link disabled)"
                        .into(),
                )
                .into());
            }
            let created = db::create_user(
                &state.pool,
                email,
                None,
                name.as_deref(),
                Some(sub),
                Some(&issuer),
            )
            .await?;
            match db::link_pending_share_invites(&state.pool, email, created.id).await {
                Ok(n) if n > 0 => {
                    tracing::info!(user_id = %created.id, linked = n, "linked pending share invites")
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(user_id = %created.id, error = %e, "pending share link failed")
                }
            }
            created
        }
    };

    let token = db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
    let cookie = session_cookie(&token, state.cfg.session_days, state.cfg.cookie_secure());
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let dest = if settings.default_landing.as_str() == "/library" {
        "/library"
    } else {
        "/"
    };
    let mut res = Redirect::temporary(dest).into_response();
    res.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| Error::msg(e.to_string()))?,
    );
    Ok(res)
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

fn session_cookie(token: &str, days: i64, secure: bool) -> String {
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

fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
