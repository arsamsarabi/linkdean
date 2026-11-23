//! Per-organization OpenID Connect sign-in (Organizations Phase D1).
//!
//! Admins store one IdP per organization. Members discover it by work email domain (must be a
//! verified domain of that org), then complete the same authorization-code + PKCE + nonce flow
//! as instance OIDC, with a flow cookie scoped to these routes.

use super::session_cookie;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use linkdean_core::crypto::{constant_time_eq, decrypt_org_sso_secret};
use linkdean_core::legal::require_current_terms;
use linkdean_core::{DiscoverOrgSsoRequest, DiscoverOrgSsoResponse, Error, OrganizationRole, User};
use linkdean_db::{self as db};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

const FLOW_COOKIE: &str = "linkdean_org_oidc";
const FLOW_COOKIE_PATH: &str = "/api/auth/org-oidc";
const FLOW_MAX_AGE_SECS: u32 = 600;

fn redirect_url(state: &AppState) -> String {
    format!(
        "{}/api/auth/org-oidc/callback",
        state.cfg.app_url.trim_end_matches('/')
    )
}

fn http_client() -> Result<openidconnect::reqwest::Client, Error> {
    openidconnect::reqwest::ClientBuilder::new()
        .redirect(openidconnect::reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| Error::msg(e.to_string()))
}

#[derive(Debug, PartialEq)]
struct FlowCookie {
    state: String,
    nonce: String,
    verifier: String,
    organization_id: Uuid,
    terms: Option<String>,
}

impl FlowCookie {
    /// `state.nonce.verifier.organization_id` plus optional `.terms.<version>`.
    fn encode(&self) -> String {
        let base = format!(
            "{}.{}.{}.{}",
            self.state, self.nonce, self.verifier, self.organization_id
        );
        match &self.terms {
            Some(v) => format!("{base}.terms.{v}"),
            None => base,
        }
    }

    fn decode(raw: &str) -> Option<Self> {
        let parts: Vec<&str> = raw.split('.').collect();
        let (org_raw, terms) = match parts.as_slice() {
            [_, _, _, org] => (*org, None),
            [_, _, _, org, "terms", v] if !v.is_empty() => (*org, Some((*v).to_string())),
            _ => return None,
        };
        let organization_id = org_raw.parse().ok()?;
        let (state, nonce, verifier) = (parts[0], parts[1], parts[2]);
        if [state, nonce, verifier].iter().any(|p| p.is_empty()) {
            return None;
        }
        Some(Self {
            state: state.into(),
            nonce: nonce.into(),
            verifier: verifier.into(),
            organization_id,
            terms,
        })
    }
}

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

fn flow_cookie_header(value: &str, max_age: u32, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
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

fn error_redirect(reason: &str) -> Response {
    let q: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("oidc_error", reason)
        .finish();
    Redirect::to(&format!("/login?{q}")).into_response()
}

fn reason(e: &Error) -> String {
    match e {
        Error::BadRequest(m) | Error::Conflict(m) => m.clone(),
        Error::Forbidden => "registration is closed".to_string(),
        Error::Banned { .. } => "this account is banned".to_string(),
        other => {
            tracing::error!(error = %other, "organization OIDC sign-in failed");
            "sign-in failed".to_string()
        }
    }
}

struct Identity {
    issuer: String,
    subject: String,
    email: String,
    email_verified: bool,
    name: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct StartQuery {
    organization_id: Uuid,
    terms_version: Option<String>,
}

#[utoipa::path(
    post,
    operation_id = "auth_org_sso_discover",
    path = "/auth/org-sso/discover",
    tag = "auth",
    request_body = DiscoverOrgSsoRequest,
    responses(
        (status = 200, description = "OK", body = DiscoverOrgSsoResponse),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
pub(super) async fn discover(
    State(state): State<AppState>,
    Json(req): Json<DiscoverOrgSsoRequest>,
) -> ApiResult<Json<DiscoverOrgSsoResponse>> {
    Ok(Json(
        db::discover_organization_sso(&state.pool, &req.email).await?,
    ))
}

#[utoipa::path(
    get,
    operation_id = "auth_org_oidc_start",
    path = "/auth/org-oidc/start",
    tag = "auth",
    params(StartQuery),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
pub(super) async fn start(
    State(state): State<AppState>,
    Query(q): Query<StartQuery>,
) -> ApiResult<Response> {
    require_current_terms(state.cfg.terms_version(), q.terms_version.as_deref())?;
    let terms = state.cfg.terms_version().map(str::to_string);

    let row = db::get_enabled_organization_sso(&state.pool, q.organization_id).await?;
    let client_secret = decrypt_org_sso_secret(
        &state.cfg.session_secret,
        &row.client_secret_ciphertext,
        &row.client_secret_nonce,
    )?;

    let http = http_client()?;
    let issuer = IssuerUrl::new(row.issuer_url.clone())
        .map_err(|e| Error::BadRequest(format!("OIDC issuer is not a URL: {e}")))?;
    let meta = CoreProviderMetadata::discover_async(issuer, &http)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, issuer = %row.issuer_url, "org OIDC discovery failed");
            Error::msg("OIDC discovery failed")
        })?;
    let redirect = RedirectUrl::new(redirect_url(&state)).map_err(|e| Error::msg(e.to_string()))?;
    let client = CoreClient::from_provider_metadata(
        meta,
        ClientId::new(row.client_id.clone()),
        Some(ClientSecret::new(client_secret)),
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
        organization_id: q.organization_id,
        terms,
    };
    let mut res = Redirect::to(url.as_str()).into_response();
    set_cookie(
        &mut res,
        &flow_cookie_header(&flow.encode(), FLOW_MAX_AGE_SECS, state.cfg.cookie_secure()),
    )?;
    Ok(res)
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[utoipa::path(
    get,
    operation_id = "auth_org_oidc_callback",
    path = "/auth/org-oidc/callback",
    tag = "auth",
    params(CallbackQuery),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
pub(super) async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> ApiResult<Response> {
    let secure = state.cfg.cookie_secure();
    let mut res = match sign_in(&state, read_flow_cookie(&headers), q).await {
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
        Err(e) => error_redirect(&reason(&e)),
    };
    set_cookie(&mut res, &flow_cookie_header("", 0, secure))?;
    Ok(res)
}

async fn sign_in(
    state: &AppState,
    flow: Option<FlowCookie>,
    q: CallbackQuery,
) -> Result<User, Error> {
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

    let row = db::get_enabled_organization_sso(&state.pool, flow.organization_id).await?;
    let client_secret = decrypt_org_sso_secret(
        &state.cfg.session_secret,
        &row.client_secret_ciphertext,
        &row.client_secret_nonce,
    )?;

    let http = http_client()?;
    let issuer = IssuerUrl::new(row.issuer_url.clone())
        .map_err(|e| Error::BadRequest(format!("OIDC issuer is not a URL: {e}")))?;
    let meta = CoreProviderMetadata::discover_async(issuer, &http)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "org OIDC discovery failed");
            Error::msg("OIDC discovery failed")
        })?;
    let redirect = RedirectUrl::new(redirect_url(state)).map_err(|e| Error::msg(e.to_string()))?;
    let client = CoreClient::from_provider_metadata(
        meta,
        ClientId::new(row.client_id.clone()),
        Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(redirect);

    let tokens = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| Error::msg(e.to_string()))?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.verifier.clone()))
        .request_async(&http)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "org OIDC token exchange failed");
            Error::BadRequest("token exchange failed".into())
        })?;
    let id_token = tokens
        .id_token()
        .ok_or_else(|| Error::BadRequest("provider sent no ID token".into()))?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &Nonce::new(flow.nonce.clone()))
        .map_err(|e| {
            tracing::warn!(error = %e, "org OIDC ID token rejected");
            Error::BadRequest("ID token rejected".into())
        })?
        .clone();
    let sub = claims.subject().clone();
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
                    tracing::error!(error = %e, "org OIDC userinfo failed");
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
    let id = Identity {
        issuer: row.issuer_url.clone(),
        subject: sub.as_str().to_string(),
        email,
        email_verified: email_verified == Some(true),
        name,
    };

    if !db::email_matches_org_verified_domain(&state.pool, flow.organization_id, &id.email).await? {
        return Err(Error::BadRequest(
            "your email domain is not verified for this organization".into(),
        ));
    }

    let user = resolve_user(state, &flow, &id).await?;
    if let Some(ban) = db::active_ban(&state.pool, user.id).await? {
        return Err(Error::Banned {
            reason: ban.reason,
            ends_at: ban.ends_at,
        });
    }

    match db::add_organization_member(
        &state.pool,
        flow.organization_id,
        user.id,
        OrganizationRole::Member,
    )
    .await
    {
        Ok(_) | Err(Error::Conflict(_)) => {}
        Err(e) => return Err(e),
    }
    if let Err(e) = db::record_sso_login(&state.pool, flow.organization_id, user.id).await {
        tracing::warn!(error = %e, "recording organization SSO login audit failed");
    }
    Ok(user)
}

async fn resolve_user(state: &AppState, flow: &FlowCookie, id: &Identity) -> Result<User, Error> {
    if let Some(user) = db::find_user_by_oidc(&state.pool, &id.issuer, &id.subject).await? {
        if user.email_verified_at.is_none()
            && id.email_verified
            && id.email.trim().eq_ignore_ascii_case(&user.email)
        {
            mark_verified(state, user.id).await;
        }
        return Ok(user);
    }

    if let Some(holder) = db::find_user_by_email(&state.pool, &id.email).await? {
        if let Some(existing_issuer) = holder.oidc_issuer.as_deref() {
            if existing_issuer != id.issuer.as_str() {
                return Err(Error::Conflict(
                    "this email is already linked to a different single sign-on provider; \
                     disconnect it first or use another account"
                        .into(),
                ));
            }
            if holder.oidc_subject.as_deref() != Some(id.subject.as_str()) {
                return Err(Error::Conflict(
                    "this email is linked to a different single sign-on identity".into(),
                ));
            }
            return Ok(holder);
        }
        // Password (or passkey) account for a verified org domain: attach IdP identity.
        db::link_oidc_identity(&state.pool, holder.id, &id.issuer, &id.subject).await?;
        if holder.email_verified_at.is_none() && id.email_verified {
            mark_verified(state, holder.id).await;
        }
        return db::find_user_by_id(&state.pool, holder.id)
            .await?
            .ok_or(Error::NotFound);
    }

    require_current_terms(state.cfg.terms_version(), flow.terms.as_deref())?;
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
            terms_version: state.cfg.terms_version(),
            plan: state.cfg.managed_service.then_some("free"),
        },
    )
    .await?;
    if id.email_verified {
        mark_verified(state, created.id).await;
    }
    Ok(created)
}

async fn mark_verified(state: &AppState, user_id: Uuid) {
    match db::mark_email_verified(&state.pool, user_id).await {
        Ok(n) if n > 0 => {
            tracing::info!(%user_id, linked = n, "linked pending share invites")
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(%user_id, error = %e, "marking email verified failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_cookie_round_trips() {
        let org = Uuid::nil();
        let flow = FlowCookie {
            state: "s-_1".into(),
            nonce: "n".into(),
            verifier: "v".into(),
            organization_id: org,
            terms: None,
        };
        assert_eq!(flow.encode(), format!("s-_1.n.v.{org}"));
        assert_eq!(FlowCookie::decode(&flow.encode()), Some(flow));
    }

    #[test]
    fn flow_cookie_carries_terms() {
        let org = Uuid::nil();
        let flow = FlowCookie {
            state: "s".into(),
            nonce: "n".into(),
            verifier: "v".into(),
            organization_id: org,
            terms: Some("2026-11-15".into()),
        };
        let decoded = FlowCookie::decode(&flow.encode()).unwrap();
        assert_eq!(decoded.terms.as_deref(), Some("2026-11-15"));
        assert_eq!(decoded.organization_id, org);
    }
}
