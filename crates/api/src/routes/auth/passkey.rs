//! Passkeys: add them to an account, and sign in with one without typing an email.
//!
//! Each WebAuthn ceremony is two calls. `start` returns options for `navigator.credentials` and
//! a `challenge_id`; the ceremony state stays server side (`webauthn_challenges`), single use,
//! for five minutes. `finish` takes the browser's answer with that id.
//!
//! Sign-in uses discoverable credentials: the passkey carries the account id (the WebAuthn user
//! handle is `users.id`), so the browser offers the passkeys it holds for this site and nobody
//! types an address. Adding a passkey needs the password, or a fresh single sign-on for an
//! account without one, so a stolen session cannot plant a lasting way in.

use super::session_response;
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::mail::enqueue;
use crate::rate_limit::{client_ip, login_limits, passkey_login_limit};
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use linkdean_core::crypto::verify_password;
use linkdean_core::{AuthResponse, Error};
use linkdean_db::{self as db, WebauthnPurpose};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CredentialID, DiscoverableAuthentication, Passkey, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, Url, Webauthn, WebauthnBuilder,
};

/// The relying party for this instance: `APP_URL`'s host and origin. Passkeys only work on the
/// address they were made on, so changing `APP_URL` strands them.
pub(crate) fn webauthn(state: &AppState) -> Result<Webauthn, Error> {
    let origin = Url::parse(&state.cfg.app_url)
        .map_err(|e| Error::msg(format!("APP_URL is not a URL: {e}")))?;
    let rp_id = origin
        .host_str()
        .ok_or_else(|| Error::msg("APP_URL has no host"))?
        .to_string();
    WebauthnBuilder::new(&rp_id, &origin)
        .and_then(|b| b.rp_name("Linkdean").build())
        .map_err(|e| Error::msg(format!("passkeys unavailable: {e}")))
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<Value, Error> {
    serde_json::to_value(v).map_err(|e| Error::msg(e.to_string()))
}

fn from_json<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, Error> {
    serde_json::from_value(v).map_err(|e| Error::msg(e.to_string()))
}

/// Sign-in finds the account from the passkey itself, so the browser must make a discoverable
/// one. webauthn-rs asks for a non-discoverable credential; the server-side check does not
/// depend on it, so the options are changed on the way out.
fn require_discoverable(mut options: Value) -> Value {
    let sel = &mut options["publicKey"]["authenticatorSelection"];
    if !sel.is_object() {
        *sel = json!({});
    }
    sel["residentKey"] = json!("required");
    sel["requireResidentKey"] = json!(true);
    options
}

/// The refusal for any failed passkey sign-in. It never says which part failed.
fn sign_in_failed() -> Error {
    Error::Unauthorized
}

#[derive(Deserialize)]
pub(super) struct RegisterStart {
    /// Required when the account has a password.
    #[serde(default)]
    password: Option<String>,
}

/// Starts adding a passkey to the signed-in account. Login sessions only, not API keys.
pub(super) async fn register_start(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    session: VaultSession,
    Json(body): Json<RegisterStart>,
) -> ApiResult<Json<Value>> {
    let Some(session_hash) = session.token_hash() else {
        return Err(Error::Forbidden.into());
    };
    match user.password_hash.clone() {
        Some(hash) => {
            let password = body
                .password
                .filter(|p| !p.is_empty())
                .ok_or_else(|| Error::BadRequest("password required".into()))?;
            // Wrong passwords count against the sign-in limits, as at sign-in.
            let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
            let limits = login_limits(ip, &user.email);
            state.limiter.check(&limits)?;
            let checked = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
                .await
                .map_err(|e| Error::msg(e.to_string()))
                .and_then(|r| r)
                .and_then(|ok| if ok { Ok(()) } else { Err(Error::Unauthorized) });
            state.limiter.settle(&limits, checked)?;
        }
        None => {
            if !db::session_recently_reauthenticated(&state.pool, user.id, session_hash).await? {
                return Err(Error::BadRequest(db::REAUTH_REQUIRED.into()).into());
            }
        }
    }
    let exclude: Vec<CredentialID> = db::passkey_credential_ids(&state.pool, user.id)
        .await?
        .into_iter()
        .map(CredentialID::from)
        .collect();
    let display = user
        .display_name
        .clone()
        .unwrap_or_else(|| user.email.clone());
    let (options, reg) = webauthn(&state)?
        .start_passkey_registration(user.id, &user.email, &display, Some(exclude))
        .map_err(|e| Error::msg(e.to_string()))?;
    let options = require_discoverable(to_json(&options)?);
    let id = db::create_webauthn_challenge(
        &state.pool,
        Some(user.id),
        WebauthnPurpose::Register,
        &to_json(&reg)?,
    )
    .await?;
    Ok(Json(json!({ "challenge_id": id, "options": options })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_options_ask_for_a_discoverable_passkey() {
        let out = require_discoverable(json!({
            "publicKey": { "authenticatorSelection": { "residentKey": "discouraged", "requireResidentKey": false, "userVerification": "required" } }
        }));
        let sel = &out["publicKey"]["authenticatorSelection"];
        assert_eq!(sel["residentKey"], "required");
        assert_eq!(sel["requireResidentKey"], true);
        assert_eq!(sel["userVerification"], "required");
        let bare = require_discoverable(json!({ "publicKey": {} }));
        assert_eq!(
            bare["publicKey"]["authenticatorSelection"]["residentKey"],
            "required"
        );
    }
}

#[derive(Deserialize)]
pub(super) struct RegisterFinish {
    challenge_id: Uuid,
    name: String,
    credential: RegisterPublicKeyCredential,
}

/// Saves the passkey the browser made, and tells the account's address.
pub(super) async fn register_finish(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<RegisterFinish>,
) -> ApiResult<Json<Value>> {
    let reg: PasskeyRegistration = from_json(
        db::take_webauthn_challenge(
            &state.pool,
            body.challenge_id,
            WebauthnPurpose::Register,
            Some(user.id),
        )
        .await?
        .ok_or_else(|| Error::BadRequest("that request expired; try again".into()))?,
    )?;
    let passkey = webauthn(&state)?
        .finish_passkey_registration(&body.credential, &reg)
        .map_err(|e| {
            tracing::info!(user_id = %user.id, error = %e, "passkey registration refused");
            Error::BadRequest("the passkey could not be checked; try again".into())
        })?;
    let summary = db::insert_passkey(
        &state.pool,
        user.id,
        passkey.cred_id().as_ref(),
        &body.name,
        &to_json(&passkey)?,
    )
    .await?;
    tracing::info!(user_id = %user.id, "passkey added");
    if let Err(e) = enqueue(
        &state,
        &user.email,
        "passkey_added",
        &json!({ "name": summary.name }),
    )
    .await
    {
        tracing::warn!(user_id = %user.id, error = %e, "passkey added email failed");
    }
    Ok(Json(json!({ "passkey": summary })))
}

pub(super) async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let passkeys = db::list_passkeys(&state.pool, user.id).await?;
    Ok(Json(json!({ "passkeys": passkeys })))
}

#[derive(Deserialize)]
pub(super) struct Rename {
    name: String,
}

pub(super) async fn rename(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<Rename>,
) -> ApiResult<Json<Value>> {
    db::rename_passkey(&state.pool, user.id, id, &body.name).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Removes a passkey. Passkeys are never an account's only way in, so no check is needed.
pub(super) async fn remove(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_passkey(&state.pool, user.id, id).await?;
    tracing::info!(user_id = %user.id, "passkey removed");
    Ok(Json(json!({ "ok": true })))
}

/// Starts a passkey sign-in. Anyone may ask; it costs a row that expires in minutes.
pub(super) async fn login_start(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    state.limiter.check(&passkey_login_limit(ip))?;
    let (mut options, auth) = webauthn(&state)?
        .start_discoverable_authentication()
        .map_err(|e| Error::msg(e.to_string()))?;
    // A button press, not autofill: let the browser show its usual passkey dialog.
    options.mediation = None;
    let id =
        db::create_webauthn_challenge(&state.pool, None, WebauthnPurpose::Login, &to_json(&auth)?)
            .await?;
    Ok(Json(json!({ "challenge_id": id, "options": options })))
}

#[derive(Deserialize)]
pub(super) struct LoginFinish {
    challenge_id: Uuid,
    credential: PublicKeyCredential,
}

/// Checks the passkey's answer and signs in. Failures count against the address's sign-in limit.
pub(super) async fn login_finish(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<LoginFinish>,
) -> ApiResult<impl IntoResponse> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = passkey_login_limit(ip);
    state.limiter.check(&limits)?;
    let result = verify_login(&state, body).await;
    let user = state.limiter.settle(&limits, result)?;
    let token = db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
    let public = db::to_public(&state.pool, &user).await?;
    let body = AuthResponse {
        user: public,
        token: token.clone(),
    };
    Ok(session_response(
        body,
        &token,
        state.cfg.session_days,
        state.cfg.cookie_secure(),
    ))
}

async fn verify_login(state: &AppState, body: LoginFinish) -> Result<linkdean_core::User, Error> {
    let auth: DiscoverableAuthentication = from_json(
        db::take_webauthn_challenge(&state.pool, body.challenge_id, WebauthnPurpose::Login, None)
            .await?
            .ok_or_else(|| Error::BadRequest("that sign-in expired; try again".into()))?,
    )?;
    let wa = webauthn(state)?;
    let (user_id, cred_id) = wa
        .identify_discoverable_authentication(&body.credential)
        .map_err(|_| sign_in_failed())?;
    let (row_id, stored) = db::find_passkey(&state.pool, user_id, cred_id)
        .await?
        .ok_or_else(sign_in_failed)?;
    let mut passkey: Passkey = from_json(stored)?;
    let result = wa
        .finish_discoverable_authentication_v2(&body.credential, auth, &passkey)
        .map_err(|e| {
            tracing::info!(%user_id, error = %e, "passkey sign-in refused");
            sign_in_failed()
        })?;
    passkey.update_credential(&result);
    db::record_passkey_use(&state.pool, row_id, &to_json(&passkey)?).await?;
    let user = db::find_user_by_id(&state.pool, user_id)
        .await?
        .ok_or_else(sign_in_failed)?;
    tracing::info!(user_id = %user.id, "signed in with a passkey");
    Ok(user)
}
