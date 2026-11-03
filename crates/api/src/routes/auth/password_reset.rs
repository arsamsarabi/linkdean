//! Forgotten passwords: request a link by email, then set a new password with it.

use super::session_response;
use crate::error::ApiResult;
use crate::mail::{enqueue, Queued};
use crate::rate_limit::{client_ip, reset_attempt_limit, reset_request_limits};
use crate::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use linkdean_core::{AuthResponse, Error};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;

/// How long an emailed link works.
pub const EMAIL_LINK_MINUTES: i64 = 30;

#[derive(Deserialize)]
pub struct ForgotRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetRequest {
    pub token: String,
    pub password: String,
    #[serde(default)]
    pub revoke_api_keys: bool,
}

/// The web page that takes a reset token.
pub fn reset_url(app_url: &str, token: &str) -> String {
    // The token rides in the fragment, which browsers never send to a server.
    format!(
        "{}/reset-password#token={token}",
        app_url.trim_end_matches('/')
    )
}

/// Always answers the same, at once; the lookup and email happen in the background, so neither
/// the reply nor its timing says whether the address has an account.
pub async fn forgot(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ForgotRequest>,
) -> ApiResult<Json<Value>> {
    if state.mail.is_none() {
        return Err(Error::BadRequest(
            "password reset by email is off; ask your admin for a reset link".into(),
        )
        .into());
    }
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = reset_request_limits(ip, &req.email);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let email = req.email.trim().to_ascii_lowercase();
    if !email.is_empty() {
        tokio::spawn(async move {
            if let Err(e) = send_reset_email(&state, &email).await {
                tracing::warn!(error = %e, "password reset email failed");
            }
        });
    }
    Ok(Json(json!({ "ok": true, "minutes": EMAIL_LINK_MINUTES })))
}

async fn send_reset_email(state: &AppState, email: &str) -> linkdean_core::Result<()> {
    let Some(user) = db::find_user_by_email(&state.pool, email).await? else {
        return Ok(());
    };
    if user.password_hash.is_none() {
        enqueue(state, &user.email, "password_reset_sso", &json!({})).await?;
        return Ok(());
    }
    let token = db::create_reset_token(
        &state.pool,
        user.id,
        chrono::Duration::minutes(EMAIL_LINK_MINUTES),
        None,
    )
    .await?;
    let data = json!({
        "reset_url": reset_url(&state.cfg.app_url, &token),
        "minutes": EMAIL_LINK_MINUTES,
    });
    if enqueue(state, &user.email, "password_reset", &data).await? == Queued::Disabled {
        tracing::warn!("password reset requested while mail is off");
    }
    Ok(())
}

/// Set the new password, end every session and sign this browser in.
pub async fn reset(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = reset_attempt_limit(ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let out = db::consume_reset_token(
        &state.pool,
        req.token.trim(),
        &req.password,
        req.revoke_api_keys,
    )
    .await?;
    // A new password locks every vault, in every session.
    state.clear_vault_keys(out.user_id).await;
    let user = db::find_user_by_id(&state.pool, out.user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let changed_at = chrono::Utc::now()
        .format("%-d %b %Y, %H:%M UTC")
        .to_string();
    if let Err(e) = enqueue(
        &state,
        &user.email,
        "password_changed",
        &json!({ "changed_at": changed_at }),
    )
    .await
    {
        tracing::warn!(error = %e, "password changed email failed");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_url_puts_the_token_in_the_fragment() {
        assert_eq!(
            reset_url("https://links.example.com", "abc"),
            "https://links.example.com/reset-password#token=abc"
        );
    }

    #[test]
    fn reset_url_has_no_double_slash() {
        assert_eq!(
            reset_url("https://links.example.com/", "abc"),
            "https://links.example.com/reset-password#token=abc"
        );
        assert_eq!(
            reset_url("https://example.com/links//", "abc"),
            "https://example.com/links/reset-password#token=abc"
        );
    }
}
