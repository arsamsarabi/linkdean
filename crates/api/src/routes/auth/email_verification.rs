//! Email verification and email change. Both links put the token in the fragment and work
//! without a session: the token proves control of the inbox.

use super::password_reset::fragment_link;
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::mail::{enqueue, Queued};
use crate::rate_limit::{client_ip, verify_attempt_limit, verify_resend_limits};
use crate::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use linkdean_core::Error;
use linkdean_db::{self as db, TokenPurpose, EMAIL_VERIFY_LINK_HOURS};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use uuid::Uuid;

const MAIL_OFF: &str = "email verification needs outbound email; ask your admin";
const CHANGE_MAIL_OFF: &str = "changing your email needs outbound email; ask your admin";

#[derive(Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ChangeRequest {
    pub password: String,
    pub new_email: String,
}

/// The web page that takes a verification token.
pub fn verify_url(app_url: &str, token: &str) -> String {
    fragment_link(app_url, "verify-email", token)
}

/// The web page that takes an email change token.
pub fn change_url(app_url: &str, token: &str) -> String {
    format!("{}&change=1", fragment_link(app_url, "verify-email", token))
}

/// `nora@example.com` → `n•••@example.com`: enough to recognize, not enough to harvest.
pub fn mask_email(email: &str) -> String {
    let (local, domain) = email.rsplit_once('@').unwrap_or((email, ""));
    let first = local.chars().next().map(String::from).unwrap_or_default();
    if domain.is_empty() {
        return format!("{first}•••");
    }
    format!("{first}•••@{domain}")
}

/// Queue a fresh verification email for `user_id` at `email`. The new link voids the last one.
pub async fn send_verify_email(
    state: &AppState,
    user_id: Uuid,
    email: &str,
) -> linkdean_core::Result<Queued> {
    if state.mail.is_none() {
        return Ok(Queued::Disabled);
    }
    let token = db::create_account_token(
        &state.pool,
        user_id,
        TokenPurpose::VerifyEmail,
        chrono::Duration::hours(EMAIL_VERIFY_LINK_HOURS),
        None,
        None,
    )
    .await?;
    let data = json!({
        "verify_url": verify_url(&state.cfg.app_url, &token),
        "hours": EMAIL_VERIFY_LINK_HOURS,
    });
    enqueue(state, email, "verify_email", &data).await
}

fn limit_by_ip(state: &AppState, peer: SocketAddr, headers: &HeaderMap) -> ApiResult<()> {
    let ip = client_ip(headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = verify_attempt_limit(ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    Ok(())
}

fn limit_resend(
    state: &AppState,
    user_id: Uuid,
    peer: SocketAddr,
    headers: &HeaderMap,
) -> ApiResult<()> {
    let ip = client_ip(headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = verify_resend_limits(user_id, ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    Ok(())
}

/// Send a new verification link to the signed-in account's address.
pub async fn resend(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    if user.email_verified_at.is_some() {
        return Ok(Json(json!({ "ok": true, "verified": true })));
    }
    if state.mail.is_none() {
        return Err(Error::BadRequest(MAIL_OFF.into()).into());
    }
    limit_resend(&state, user.id, peer, &headers)?;
    send_verify_email(&state, user.id, &user.email).await?;
    Ok(Json(json!({
        "ok": true,
        "verified": false,
        "hours": EMAIL_VERIFY_LINK_HOURS,
    })))
}

/// Verify an address with its link. No session needed.
pub async fn verify(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<TokenRequest>,
) -> ApiResult<Json<Value>> {
    limit_by_ip(&state, peer, &headers)?;
    db::verify_email_token(&state.pool, req.token.trim()).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Start an email change: the link goes to the new address, a notice to the old one. The
/// address stays the same until the link is opened.
pub async fn change(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    Json(req): Json<ChangeRequest>,
) -> ApiResult<Json<Value>> {
    if state.mail.is_none() {
        return Err(Error::BadRequest(CHANGE_MAIL_OFF.into()).into());
    }
    crate::disposable_email::refuse_if_disposable(
        &req.new_email,
        state.cfg.block_disposable_emails,
    )?;
    limit_resend(&state, user.id, peer, &headers)?;
    let token = db::start_email_change(&state.pool, user.id, &req.password, &req.new_email).await?;
    let new_email = req.new_email.trim().to_ascii_lowercase();
    let data = json!({
        "verify_url": change_url(&state.cfg.app_url, &token),
        "hours": EMAIL_VERIFY_LINK_HOURS,
    });
    enqueue(&state, &new_email, "email_change_verify", &data).await?;
    let notice = json!({ "masked_email": mask_email(&new_email) });
    if let Err(e) = enqueue(&state, &user.email, "email_change_notice", &notice).await {
        tracing::warn!(user_id = %user.id, error = %e, "email change notice failed");
    }
    Ok(Json(
        json!({ "ok": true, "hours": EMAIL_VERIFY_LINK_HOURS }),
    ))
}

/// Finish an email change with its link. No session needed.
pub async fn confirm_change(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<TokenRequest>,
) -> ApiResult<Json<Value>> {
    limit_by_ip(&state, peer, &headers)?;
    let (user_id, _old, _new) = db::confirm_email_change(&state.pool, req.token.trim()).await?;
    tracing::info!(%user_id, "email changed");
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_put_the_token_in_the_fragment() {
        assert_eq!(
            verify_url("https://links.example.com/", "abc"),
            "https://links.example.com/verify-email#token=abc"
        );
        assert_eq!(
            change_url("https://links.example.com", "abc"),
            "https://links.example.com/verify-email#token=abc&change=1"
        );
    }

    #[test]
    fn mask_keeps_the_first_letter_and_the_domain() {
        assert_eq!(mask_email("nora@example.com"), "n•••@example.com");
        assert_eq!(mask_email("a@b.co"), "a•••@b.co");
        assert_eq!(mask_email("odd"), "o•••");
    }
}
