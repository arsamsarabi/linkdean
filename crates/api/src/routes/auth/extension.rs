//! Signing the browser extension in, PKCE style. The extension opens `/connect/extension` on
//! the web app in a window of its own; the person signs in as usual and approves; `authorize`
//! returns a single-use code to that page, which posts it to a small script the extension
//! placed on the page; the extension trades the code and its verifier at `token` for an API
//! key. The person never sees the key, and it is listed and revocable under Settings → API keys
//! like any other.

use crate::auth::SessionUser;
use crate::error::ApiResult;
use crate::rate_limit::{client_ip, extension_connect_limit, extension_token_limit};
use crate::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use linkdean_core::extension_connect::{extension_browser, valid_challenge, valid_verifier};
use linkdean_core::{CreateApiKey, Error};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;

const SPENT: &str =
    "This sign-in has expired or was already used. Sign in from the extension again.";

#[derive(Deserialize)]
pub struct AuthorizeRequest {
    /// The extension's base address, `chrome-extension://<id>/` or `moz-extension://<uuid>/`.
    pub extension_origin: String,
    pub code_challenge: String,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

/// Login sessions only, like creating any API key: a key must never mint another.
pub async fn authorize(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(req): Json<AuthorizeRequest>,
) -> ApiResult<Json<Value>> {
    let Some(browser) = extension_browser(&req.extension_origin) else {
        return Err(Error::BadRequest(
            "this sign-in did not come from the Linkdean extension".into(),
        )
        .into());
    };
    if req.code_challenge_method.as_deref().unwrap_or("S256") != "S256"
        || !valid_challenge(&req.code_challenge)
    {
        return Err(
            Error::BadRequest("the extension sent an invalid sign-in request".into()).into(),
        );
    }
    if req.state.as_deref().is_some_and(|s| s.len() > 200) {
        return Err(
            Error::BadRequest("the extension sent an invalid sign-in request".into()).into(),
        );
    }
    let limits = extension_connect_limit(user.id);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    db::require_api_keys_allowed(&state.pool, &user).await?;

    let key_name = format!("Browser extension · {browser}");
    let code = db::create_extension_code(
        &state.pool,
        user.id,
        &req.code_challenge,
        &req.extension_origin,
        &key_name,
    )
    .await?;
    Ok(Json(json!({ "code": code, "state": req.state })))
}

#[derive(Deserialize)]
pub struct TokenRequest {
    pub code: String,
    pub code_verifier: String,
    pub extension_origin: String,
}

/// No session: the extension proves itself with the verifier only it holds.
pub async fn token(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<TokenRequest>,
) -> ApiResult<Json<Value>> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = extension_token_limit(ip);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    if !valid_verifier(&req.code_verifier) {
        return Err(Error::BadRequest(SPENT.into()).into());
    }
    let Some(grant) = db::take_extension_code(
        &state.pool,
        &req.code,
        &req.code_verifier,
        &req.extension_origin,
    )
    .await?
    else {
        return Err(Error::BadRequest(SPENT.into()).into());
    };
    let Some(user) = db::find_user_by_id(&state.pool, grant.user_id).await? else {
        return Err(Error::BadRequest(SPENT.into()).into());
    };
    let created = db::create_api_key(
        &state.pool,
        &user,
        CreateApiKey {
            name: grant.key_name,
        },
    )
    .await?;
    Ok(Json(json!({
        "token": created.token,
        "key_id": created.key.id,
        "email": user.email,
    })))
}
