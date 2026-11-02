//! OpenID Connect sign-in: authorization code flow with discovery, state, nonce and PKCE.
//!
//! The state, nonce and PKCE verifier travel in a short-lived HttpOnly cookie scoped to the
//! OIDC routes, so the API keeps no per-login state. The callback checks the state against the
//! cookie (login CSRF), sends the verifier (code interception), and validates the ID token's
//! signature, issuer, audience, expiry and nonce (replay) before trusting its subject.

use super::session_cookie;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect, Response};
use linkdean_core::{Error, User};
use linkdean_db::{self as db};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use std::time::Duration;

const FLOW_COOKIE: &str = "linkdean_oidc";
const FLOW_COOKIE_PATH: &str = "/api/auth/oidc";
const FLOW_MAX_AGE_SECS: u32 = 600;

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

pub(super) async fn start(State(state): State<AppState>) -> ApiResult<Response> {
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

/// Always redirects: home with a session, or the login page with a reason.
pub(super) async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> ApiResult<Response> {
    let secure = state.cfg.cookie_secure();
    let mut res = match sign_in(&state, &headers, q).await {
        Ok(user) => {
            let token = db::create_session(&state.pool, user.id, state.cfg.session_days).await?;
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
        Err(e) => {
            let reason = match &e {
                Error::BadRequest(m) | Error::Conflict(m) => m.clone(),
                Error::Forbidden => "registration is closed".to_string(),
                Error::Banned { .. } => "this account is banned".to_string(),
                other => {
                    tracing::error!(error = %other, "OIDC sign-in failed");
                    "sign-in failed".to_string()
                }
            };
            let q: String = url::form_urlencoded::Serializer::new(String::new())
                .append_pair("oidc_error", &reason)
                .finish();
            Redirect::to(&format!("/login?{q}")).into_response()
        }
    };
    // The flow cookie is single use, whatever the outcome.
    res.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&flow_cookie("", 0, secure))
            .map_err(|e| Error::msg(e.to_string()))?,
    );
    Ok(res)
}

async fn sign_in(state: &AppState, headers: &HeaderMap, q: CallbackQuery) -> Result<User, Error> {
    if let Some(err) = q.error {
        return Err(Error::BadRequest(format!(
            "provider refused sign-in: {err}"
        )));
    }
    let flow = read_flow_cookie(headers)
        .ok_or_else(|| Error::BadRequest("sign-in expired; try again".into()))?;
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

    // Never auto-login by email alone (account takeover). Match the OIDC subject or create.
    if let Some(user) = db::find_user_by_oidc(&state.pool, &s.issuer, sub.as_str()).await? {
        if let Some(ban) = db::active_ban(&state.pool, user.id).await? {
            return Err(Error::Banned {
                reason: ban.reason,
                ends_at: ban.ends_at,
            });
        }
        return Ok(user);
    }
    if db::count_users(&state.pool).await? > 0
        && !db::get_platform_settings(&state.pool)
            .await?
            .registration_open
    {
        return Err(Error::Forbidden);
    }
    if db::find_user_by_email(&state.pool, &email).await?.is_some() {
        return Err(Error::Conflict(
            "email already registered; sign in with your password".into(),
        ));
    }
    let created = db::create_user(
        &state.pool,
        &email,
        None,
        name.as_deref(),
        Some(sub.as_str()),
        Some(&s.issuer),
    )
    .await?;
    if email_verified == Some(true) {
        match db::link_pending_share_invites(&state.pool, &email, created.id).await {
            Ok(n) if n > 0 => {
                tracing::info!(user_id = %created.id, linked = n, "linked pending share invites")
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(user_id = %created.id, error = %e, "pending share link failed")
            }
        }
    }
    Ok(created)
}

#[derive(Debug, PartialEq)]
struct FlowCookie {
    state: String,
    nonce: String,
    verifier: String,
}

impl FlowCookie {
    /// The three values are URL-safe base64, so `.` cannot occur inside one.
    fn encode(&self) -> String {
        format!("{}.{}.{}", self.state, self.nonce, self.verifier)
    }

    fn decode(raw: &str) -> Option<Self> {
        let mut parts = raw.split('.');
        let (state, nonce, verifier) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || [state, nonce, verifier].iter().any(|p| p.is_empty()) {
            return None;
        }
        Some(Self {
            state: state.into(),
            nonce: nonce.into(),
            verifier: verifier.into(),
        })
    }
}

fn read_flow_cookie(headers: &HeaderMap) -> Option<FlowCookie> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|c| c.split(';'))
        .find_map(|part| {
            part.trim()
                .strip_prefix(&format!("{FLOW_COOKIE}=")[..])
                .map(str::to_string)
        })
        .and_then(|v| FlowCookie::decode(&v))
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

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
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
        };
        assert_eq!(FlowCookie::decode(&flow.encode()), Some(flow));
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
