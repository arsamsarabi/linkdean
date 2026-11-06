//! OpenID Connect sign-in: authorization code flow with discovery, state, nonce and PKCE.
//!
//! The state, nonce and PKCE verifier travel in a short-lived HttpOnly cookie scoped to the
//! OIDC routes, so the API keeps no per-login state. The callback checks the state against the
//! cookie (login CSRF), sends the verifier (code interception), and validates the ID token's
//! signature, issuer, audience, expiry and nonce (replay) before trusting its subject.
//!
//! A sign-in whose address belongs to a password account does not sign in. It leaves a pending
//! link, bound to the browser by the `linkdean_oidc_link` cookie, that the next password
//! sign-in to that account attaches. In connect mode a signed-in user attaches the identity to
//! their own account directly.

use super::email_verification::mask_email;
use super::{bearer_or_cookie, session_cookie};
use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::mail::enqueue;
use crate::rate_limit::{client_ip, login_limits};
use crate::state::AppState;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use linkdean_core::crypto::constant_time_eq;
use linkdean_core::legal::{require_current_terms, TERMS_VERSION};
use linkdean_core::{issuer_host, Error, User};
use linkdean_db::{self as db};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use uuid::Uuid;

const FLOW_COOKIE: &str = "linkdean_oidc";
const FLOW_COOKIE_PATH: &str = "/api/auth/oidc";
const FLOW_MAX_AGE_SECS: u32 = 600;
/// Holds the token of a pending link. Scoped to `/api/auth`, so the password sign-in and the
/// pending-link lookup both see it.
const LINK_COOKIE: &str = "linkdean_oidc_link";
const LINK_COOKIE_PATH: &str = "/api/auth";
const LINK_MAX_AGE_SECS: u32 = (db::PENDING_OIDC_LINK_MINUTES * 60) as u32;
/// Where a connect-mode flow returns: the profile page holds the account's sign-in settings.
const CONNECT_RETURN: &str = "/profile";

struct OidcSettings {
    issuer: String,
    client_id: String,
    client_secret: String,
    redirect: String,
}

fn settings(state: &AppState) -> Result<OidcSettings, Error> {
    let cfg = &state.cfg;
    match (
        cfg.oidc_issuer.clone(),
        cfg.oidc_client_id.clone(),
        cfg.oidc_client_secret.clone(),
    ) {
        (Some(issuer), Some(client_id), Some(client_secret)) => Ok(OidcSettings {
            issuer,
            client_id,
            client_secret,
            redirect: format!(
                "{}/api/auth/oidc/callback",
                cfg.app_url.trim_end_matches('/')
            ),
        }),
        _ => Err(Error::BadRequest("OIDC not configured".into())),
    }
}

fn http_client() -> Result<openidconnect::reqwest::Client, Error> {
    // No redirects: discovery, token and userinfo answers must come from the named endpoints.
    openidconnect::reqwest::ClientBuilder::new()
        .redirect(openidconnect::reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| Error::msg(e.to_string()))
}

async fn discover(
    s: &OidcSettings,
    http: &openidconnect::reqwest::Client,
) -> Result<CoreProviderMetadata, Error> {
    let issuer = IssuerUrl::new(s.issuer.clone())
        .map_err(|e| Error::BadRequest(format!("OIDC_ISSUER is not a URL: {e}")))?;
    CoreProviderMetadata::discover_async(issuer, http)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, issuer = %s.issuer, "OIDC discovery failed");
            Error::msg("OIDC discovery failed")
        })
}

#[derive(Deserialize)]
pub(super) struct StartQuery {
    mode: Option<String>,
    terms_version: Option<String>,
}

/// `?mode=connect` attaches the provider identity to the signed-in account instead of signing
/// in; it needs a session. Sign-in may create an account, so it needs `?terms_version=` equal to
/// the current terms version; the flow cookie carries it to the callback, which records it on a
/// new account. Existing accounts sign in unchanged. Connect mode never creates an account.
pub(super) async fn start(
    State(state): State<AppState>,
    Query(q): Query<StartQuery>,
    auth: Result<AuthUser, ApiError>,
) -> ApiResult<Response> {
    let mode = match q.mode.as_deref() {
        None | Some("") | Some("signin") => FlowMode::SignIn,
        Some("connect") => match auth {
            Ok(AuthUser(user)) => FlowMode::Connect(user.id),
            Err(_) => {
                return Ok(error_redirect(
                    "/login",
                    "sign in first to connect single sign-on",
                ))
            }
        },
        Some(_) => return Err(Error::BadRequest("unknown sign-in mode".into()).into()),
    };
    let terms = match mode {
        FlowMode::SignIn => {
            require_current_terms(q.terms_version.as_deref())?;
            Some(TERMS_VERSION.to_string())
        }
        FlowMode::Connect(_) => None,
    };
    let s = settings(&state)?;
    let http = http_client()?;
    let meta = discover(&s, &http).await?;
    let redirect = RedirectUrl::new(s.redirect.clone()).map_err(|e| Error::msg(e.to_string()))?;
    let client = CoreClient::from_provider_metadata(
        meta,
        ClientId::new(s.client_id),
        Some(ClientSecret::new(s.client_secret)),
    )
    .set_redirect_uri(redirect);
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .set_pkce_challenge(challenge)
        .url();
    let flow = FlowCookie {
        state: csrf.secret().clone(),
        nonce: nonce.secret().clone(),
        verifier: verifier.secret().clone(),
        mode,
        terms,
    };
    let mut res = Redirect::to(url.as_str()).into_response();
    set_cookie(
        &mut res,
        &flow_cookie(&flow.encode(), FLOW_MAX_AGE_SECS, state.cfg.cookie_secure()),
    )?;
    Ok(res)
}

#[derive(Deserialize)]
pub(super) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// What a sign-in-mode callback ends in.
enum SignInOutcome {
    Session(User),
    /// A password account holds the provider's address: the token of its pending link.
    PendingLink(String),
}

/// The provider identity from a validated ID token (and userinfo when needed).
struct Identity {
    issuer: String,
    subject: String,
    email: String,
    email_verified: bool,
    name: Option<String>,
}

/// Always redirects: home with a session, the login page with a pending link or a reason, or
/// back to the profile page in connect mode.
pub(super) async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> ApiResult<Response> {
    let secure = state.cfg.cookie_secure();
    let flow = read_flow_cookie(&headers);
    let mode = flow.as_ref().map_or(FlowMode::SignIn, |f| f.mode);
    let mut res = match mode {
        FlowMode::SignIn => match sign_in(&state, flow, q).await {
            Ok(SignInOutcome::Session(user)) => {
                let token =
                    db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
                let settings = db::get_user_settings(&state.pool, user.id).await?;
                let dest = if settings.default_landing.as_str() == "/library" {
                    "/library"
                } else {
                    "/"
                };
                let mut res = Redirect::to(dest).into_response();
                set_cookie(
                    &mut res,
                    &session_cookie(&token, state.cfg.session_days, secure),
                )?;
                res
            }
            Ok(SignInOutcome::PendingLink(token)) => {
                let mut res = Redirect::to("/login?link=1").into_response();
                set_cookie(&mut res, &link_cookie(&token, LINK_MAX_AGE_SECS, secure))?;
                res
            }
            Err(e) => error_redirect("/login", &reason(&e)),
        },
        FlowMode::Connect(user_id) => match connect(&state, &headers, user_id, flow, q).await {
            Ok(()) => Redirect::to(&format!("{CONNECT_RETURN}?sso=connected")).into_response(),
            Err(e) => error_redirect(CONNECT_RETURN, &reason(&e)),
        },
    };
    // The flow cookie is single use, whatever the outcome.
    set_cookie(&mut res, &flow_cookie("", 0, secure))?;
    Ok(res)
}

/// The message shown for a failed callback. Internal errors stay in the log.
fn reason(e: &Error) -> String {
    match e {
        Error::BadRequest(m) | Error::Conflict(m) => m.clone(),
        Error::Forbidden => "registration is closed".to_string(),
        Error::Banned { .. } => "this account is banned".to_string(),
        other => {
            tracing::error!(error = %other, "OIDC sign-in failed");
            "sign-in failed".to_string()
        }
    }
}

/// Redirect to the web page `path` with `oidc_error` set.
fn error_redirect(path: &str, reason: &str) -> Response {
    let q: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("oidc_error", reason)
        .finish();
    Redirect::to(&format!("{path}?{q}")).into_response()
}

/// Check the flow and the provider's answer, and return the identity it vouches for.
async fn provider_identity(
    state: &AppState,
    flow: Option<FlowCookie>,
    q: CallbackQuery,
) -> Result<Identity, Error> {
    if let Some(err) = q.error {
        return Err(Error::BadRequest(format!(
            "provider refused sign-in: {err}"
        )));
    }
    let flow = flow.ok_or_else(|| Error::BadRequest("sign-in expired; try again".into()))?;
    let returned = q.state.unwrap_or_default();
    if !constant_time_eq(returned.as_bytes(), flow.state.as_bytes()) {
        return Err(Error::BadRequest(
            "sign-in state mismatch; try again".into(),
        ));
    }
    let code = q
        .code
        .ok_or_else(|| Error::BadRequest("missing code".into()))?;
    let s = settings(state)?;
    let http = http_client()?;
    let meta = discover(&s, &http).await?;
    let redirect = RedirectUrl::new(s.redirect.clone()).map_err(|e| Error::msg(e.to_string()))?;
    let client = CoreClient::from_provider_metadata(
        meta,
        ClientId::new(s.client_id.clone()),
        Some(ClientSecret::new(s.client_secret.clone())),
    )
    .set_redirect_uri(redirect);

    let tokens = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| Error::msg(e.to_string()))?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.verifier))
        .request_async(&http)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "OIDC token exchange failed");
            Error::BadRequest("token exchange failed".into())
        })?;
    let id_token = tokens
        .id_token()
        .ok_or_else(|| Error::BadRequest("provider sent no ID token".into()))?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &Nonce::new(flow.nonce))
        .map_err(|e| {
            tracing::warn!(error = %e, "OIDC ID token rejected");
            Error::BadRequest("ID token rejected".into())
        })?
        .clone();
    let sub = claims.subject().clone();

    // Providers may leave email out of the ID token; userinfo fills it in for the same subject.
    let (mut email, mut email_verified, mut name) = (
        claims.email().map(|e| e.as_str().to_string()),
        claims.email_verified(),
        claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.as_str().to_string()),
    );
    if email.is_none() {
        if let Ok(req) = client.user_info(tokens.access_token().clone(), Some(sub.clone())) {
            let info: openidconnect::core::CoreUserInfoClaims =
                req.request_async(&http).await.map_err(|e| {
                    tracing::error!(error = %e, "OIDC userinfo failed");
                    Error::BadRequest("userinfo request failed".into())
                })?;
            email = info.email().map(|e| e.as_str().to_string());
            email_verified = info.email_verified();
            if name.is_none() {
                name = info
                    .name()
                    .and_then(|n| n.get(None))
                    .map(|n| n.as_str().to_string());
            }
        }
    }
    let email = email.ok_or_else(|| Error::BadRequest("provider sent no email".into()))?;

    Ok(Identity {
        issuer: s.issuer,
        subject: sub.as_str().to_string(),
        email,
        email_verified: email_verified == Some(true),
        name,
    })
}

async fn sign_in(
    state: &AppState,
    flow: Option<FlowCookie>,
    q: CallbackQuery,
) -> Result<SignInOutcome, Error> {
    let terms = flow.as_ref().and_then(|f| f.terms.clone());
    let id = provider_identity(state, flow, q).await?;

    // Never auto-login by email alone (account takeover). Match the OIDC subject or create.
    if let Some(user) = db::find_user_by_oidc(&state.pool, &id.issuer, &id.subject).await? {
        if let Some(ban) = db::active_ban(&state.pool, user.id).await? {
            return Err(Error::Banned {
                reason: ban.reason,
                ends_at: ban.ends_at,
            });
        }
        // The provider vouches for the address now, so an unverified account catches up.
        if user.email_verified_at.is_none()
            && id.email_verified
            && id.email.trim().eq_ignore_ascii_case(&user.email)
        {
            mark_verified(state, user.id).await;
        }
        return Ok(SignInOutcome::Session(user));
    }
    if let Some(holder) = db::find_user_by_email(&state.pool, &id.email).await? {
        // Only the account's password can approve attaching this identity to it.
        if holder.password_hash.is_none() {
            return Err(Error::Conflict(
                "email already registered to an account with a different single sign-on; \
                 ask your admin"
                    .into(),
            ));
        }
        if holder.oidc_subject.is_some() {
            return Err(Error::Conflict(
                "email already registered to an account connected to a different single \
                 sign-on; sign in with your password"
                    .into(),
            ));
        }
        let token = db::create_pending_oidc_link(
            &state.pool,
            &id.issuer,
            &id.subject,
            &id.email,
            id.email_verified,
        )
        .await?;
        return Ok(SignInOutcome::PendingLink(token));
    }
    // Only a new account needs the terms, which the sign-in start must have presented.
    require_current_terms(terms.as_deref())?;
    // A provider sign-in cannot carry the bootstrap token, so with one set the first account
    // must register with a password.
    let created = db::create_user_gated(
        &state.pool,
        &id.email,
        None,
        id.name.as_deref(),
        Some(&id.subject),
        Some(&id.issuer),
        db::SignupGate {
            enforce: true,
            invite_code: None,
            bootstrap_token_ok: state.cfg.bootstrap_token_ok(None),
            terms_version: Some(TERMS_VERSION),
        },
    )
    .await?;
    if id.email_verified {
        mark_verified(state, created.id).await;
    }
    Ok(SignInOutcome::Session(created))
}

/// Connect mode: attach the identity to the account that started the flow, which must still be
/// the one signed in. The account's address stays as it is, even when the provider's differs.
async fn connect(
    state: &AppState,
    headers: &HeaderMap,
    user_id: Uuid,
    flow: Option<FlowCookie>,
    q: CallbackQuery,
) -> Result<(), Error> {
    // The flow cookie is not signed, so the session decides whose account this is.
    let user = match bearer_or_cookie(headers) {
        Some(token) => db::user_from_token(&state.pool, &token).await?,
        None => None,
    }
    .filter(|u| u.id == user_id)
    .ok_or_else(|| Error::BadRequest("you were signed out; sign in and try again".into()))?;
    if let Some(ban) = db::active_ban(&state.pool, user.id).await? {
        return Err(Error::Banned {
            reason: ban.reason,
            ends_at: ban.ends_at,
        });
    }
    let id = provider_identity(state, flow, q).await?;
    let already = user.oidc_issuer.as_deref() == Some(id.issuer.as_str())
        && user.oidc_subject.as_deref() == Some(id.subject.as_str());
    db::link_oidc_identity(&state.pool, user.id, &id.issuer, &id.subject).await?;
    if user.email_verified_at.is_none()
        && id.email_verified
        && id.email.trim().eq_ignore_ascii_case(&user.email)
    {
        mark_verified(state, user.id).await;
    }
    if !already {
        tracing::info!(user_id = %user.id, "single sign-on connected");
        notify_sso_connected(state, user.id, &user.email, &id.issuer).await;
    }
    Ok(())
}

/// Tell the account's address that single sign-on was connected. Best effort.
pub(super) async fn notify_sso_connected(
    state: &AppState,
    user_id: Uuid,
    email: &str,
    issuer: &str,
) {
    let data = json!({ "provider": issuer_host(issuer) });
    if let Err(e) = enqueue(state, email, "sso_connected", &data).await {
        tracing::warn!(%user_id, error = %e, "sso connected email failed");
    }
}

/// The masked provider address of the browser's pending link, for the login prompt. 404 when
/// there is none or it expired or was used.
pub(super) async fn pending_link(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let token = read_link_cookie(&headers).ok_or(Error::NotFound)?;
    let link = db::peek_pending_oidc_link(&state.pool, &token)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(Json(json!({ "email": mask_email(&link.email) })))
}

#[derive(Deserialize)]
pub(super) struct DisconnectRequest {
    password: String,
}

/// Detach single sign-on after a password check. Accounts without a password cannot, so they
/// keep a way to sign in. Wrong passwords count against the sign-in limits.
pub(super) async fn disconnect(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AuthUser(user): AuthUser,
    Json(req): Json<DisconnectRequest>,
) -> ApiResult<Json<Value>> {
    let ip = client_ip(&headers, peer.ip(), state.cfg.trust_forwarded_for);
    let limits = login_limits(ip, &user.email);
    state.limiter.check(&limits)?;
    let result = db::unlink_oidc_identity(&state.pool, user.id, &req.password).await;
    state.limiter.settle(&limits, result)?;
    tracing::info!(user_id = %user.id, "single sign-on disconnected");
    let user = db::find_user_by_id(&state.pool, user.id)
        .await?
        .ok_or(Error::NotFound)?;
    let public = db::to_public(&state.pool, &user).await?;
    Ok(Json(json!({ "ok": true, "user": public })))
}

/// Mark the account verified, which also attaches invites waiting for its address. Best
/// effort: sign-in goes on if it fails.
async fn mark_verified(state: &AppState, user_id: uuid::Uuid) {
    match db::mark_email_verified(&state.pool, user_id).await {
        Ok(n) if n > 0 => {
            tracing::info!(%user_id, linked = n, "linked pending share invites")
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(%user_id, error = %e, "marking email verified failed"),
    }
}

/// What the flow is for: signing in, or attaching the identity to the signed-in account.
#[derive(Debug, PartialEq, Clone, Copy)]
enum FlowMode {
    SignIn,
    Connect(Uuid),
}

#[derive(Debug, PartialEq)]
struct FlowCookie {
    state: String,
    nonce: String,
    verifier: String,
    mode: FlowMode,
    /// Terms version presented at sign-in start; always `None` in connect mode.
    terms: Option<String>,
}

impl FlowCookie {
    /// `state.nonce.verifier`, plus `.terms.<version>` in sign-in mode or `.connect.<user id>`
    /// in connect mode. The three values are URL-safe base64, a UUID has no `.`, and the terms
    /// version is a date, so `.` cannot occur inside a part.
    fn encode(&self) -> String {
        let base = format!("{}.{}.{}", self.state, self.nonce, self.verifier);
        match (self.mode, &self.terms) {
            (FlowMode::SignIn, None) => base,
            (FlowMode::SignIn, Some(v)) => format!("{base}.terms.{v}"),
            (FlowMode::Connect(id), _) => format!("{base}.connect.{id}"),
        }
    }

    fn decode(raw: &str) -> Option<Self> {
        let parts: Vec<&str> = raw.split('.').collect();
        let (mode, terms) = match parts.as_slice() {
            [_, _, _] => (FlowMode::SignIn, None),
            [_, _, _, "terms", v] if !v.is_empty() => (FlowMode::SignIn, Some((*v).to_string())),
            [_, _, _, "connect", id] => (FlowMode::Connect(id.parse().ok()?), None),
            _ => return None,
        };
        let (state, nonce, verifier) = (parts[0], parts[1], parts[2]);
        if [state, nonce, verifier].iter().any(|p| p.is_empty()) {
            return None;
        }
        Some(Self {
            state: state.into(),
            nonce: nonce.into(),
            verifier: verifier.into(),
            mode,
            terms,
        })
    }
}

/// The value of cookie `name`, if the request has it.
fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|c| c.split(';'))
        .find_map(|part| part.trim().strip_prefix(&prefix[..]).map(str::to_string))
}

fn read_flow_cookie(headers: &HeaderMap) -> Option<FlowCookie> {
    read_cookie(headers, FLOW_COOKIE).and_then(|v| FlowCookie::decode(&v))
}

/// The pending-link token the browser holds, if any.
pub(super) fn read_link_cookie(headers: &HeaderMap) -> Option<String> {
    read_cookie(headers, LINK_COOKIE).filter(|v| !v.is_empty())
}

/// The pending-link cookie; an empty value with `max_age` 0 expires it.
pub(super) fn link_cookie(value: &str, max_age: u32, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    // Lax: the callback that sets it is the end of a top-level redirect from the provider.
    format!(
        "{LINK_COOKIE}={value}; Path={LINK_COOKIE_PATH}; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_flag}"
    )
}

fn flow_cookie(value: &str, max_age: u32, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    // Lax: the provider's redirect back is a top-level GET, which Lax still sends the cookie on.
    format!(
        "{FLOW_COOKIE}={value}; Path={FLOW_COOKIE_PATH}; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure_flag}"
    )
}

fn set_cookie(res: &mut Response, cookie: &str) -> Result<(), Error> {
    res.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(cookie).map_err(|e| Error::msg(e.to_string()))?,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_cookie_round_trips() {
        let flow = FlowCookie {
            state: "s-_1".into(),
            nonce: "n".into(),
            verifier: "v".into(),
            mode: FlowMode::SignIn,
            terms: None,
        };
        assert_eq!(flow.encode(), "s-_1.n.v");
        assert_eq!(FlowCookie::decode(&flow.encode()), Some(flow));
    }

    #[test]
    fn sign_in_flow_cookie_carries_the_terms_version() {
        let flow = FlowCookie {
            state: "s".into(),
            nonce: "n".into(),
            verifier: "v".into(),
            mode: FlowMode::SignIn,
            terms: Some(TERMS_VERSION.into()),
        };
        assert_eq!(flow.encode(), format!("s.n.v.terms.{TERMS_VERSION}"));
        assert_eq!(FlowCookie::decode(&flow.encode()), Some(flow));
        assert_eq!(FlowCookie::decode("s.n.v.terms."), None);
        assert_eq!(FlowCookie::decode("s.n.v.terms.x.y"), None);
    }

    #[test]
    fn connect_flow_cookie_carries_the_user() {
        let id = Uuid::new_v4();
        let flow = FlowCookie {
            state: "s".into(),
            nonce: "n".into(),
            verifier: "v".into(),
            mode: FlowMode::Connect(id),
            terms: None,
        };
        assert_eq!(FlowCookie::decode(&flow.encode()), Some(flow));
        for raw in [
            "s.n.v.connect".to_string(),
            "s.n.v.connect.not-a-uuid".to_string(),
            format!("s.n.v.other.{id}"),
            format!("s.n.v.connect.{id}.x"),
            format!(".n.v.connect.{id}"),
        ] {
            assert_eq!(FlowCookie::decode(&raw), None, "{raw}");
        }
    }

    #[test]
    fn link_cookie_is_scoped_to_auth_and_http_only() {
        let c = link_cookie("tok", 600, false);
        assert!(c.starts_with("linkdean_oidc_link=tok;"));
        assert!(c.contains("Path=/api/auth;"));
        assert!(c.contains("HttpOnly"));
        assert!(c.contains("SameSite=Lax"));
        assert!(c.contains("Max-Age=600"));
        assert!(!c.contains("Secure"));
        assert!(link_cookie("", 0, true).ends_with("Max-Age=0; Secure"));
    }

    #[test]
    fn reads_the_link_cookie_but_not_the_flow_cookie_as_it() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("linkdean_oidc=s.n.v; linkdean_oidc_link=abc"),
        );
        assert_eq!(read_link_cookie(&headers).as_deref(), Some("abc"));
        assert_eq!(
            read_flow_cookie(&headers).map(|f| f.state).as_deref(),
            Some("s")
        );
        let mut only_flow = HeaderMap::new();
        only_flow.insert(
            header::COOKIE,
            HeaderValue::from_static("linkdean_oidc=s.n.v"),
        );
        assert_eq!(read_link_cookie(&only_flow), None);
    }

    #[test]
    fn flow_cookie_rejects_malformed_values() {
        for raw in ["", "a.b", "a.b.c.d", "a..c", ".b.c"] {
            assert_eq!(FlowCookie::decode(raw), None, "{raw}");
        }
    }

    #[test]
    fn reads_the_flow_cookie_among_others() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("linkdean_session=abc; linkdean_oidc=s.n.v; other=1"),
        );
        let flow = read_flow_cookie(&headers).expect("flow");
        assert_eq!(flow.state, "s");
        assert_eq!(flow.nonce, "n");
        assert_eq!(flow.verifier, "v");
    }

    #[test]
    fn state_compare_needs_an_exact_match() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"", b"a"));
    }

    #[test]
    fn flow_cookie_is_scoped_and_http_only() {
        let c = flow_cookie("x", 600, true);
        assert!(c.contains("Path=/api/auth/oidc"));
        assert!(c.contains("HttpOnly"));
        assert!(c.contains("SameSite=Lax"));
        assert!(c.ends_with("; Secure"));
    }
}
