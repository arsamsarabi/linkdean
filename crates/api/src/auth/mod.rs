use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{FromRequestParts, OriginalUri};
use axum::http::request::Parts;
use linkdean_core::{Error, User};
use linkdean_db::{self as db};

pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts).ok_or(Error::SessionExpired)?;
        let user = db::user_from_token(&state.pool, &token)
            .await?
            .ok_or(Error::SessionExpired)?;
        if let Some(ban) = db::active_ban(&state.pool, user.id).await? {
            return Err(Error::Banned {
                reason: ban.reason,
                ends_at: ban.ends_at,
            }
            .into());
        }
        // Verified accounts skip the gate without another query. Admins are never gated, so
        // the setting cannot lock everyone out.
        if user.email_verified_at.is_none()
            && !user.is_admin
            && !gate_exempt(request_path(parts))
            && db::get_platform_settings(&state.pool)
                .await?
                .require_verified_email
        {
            return Err(Error::EmailUnverified.into());
        }
        Ok(AuthUser(user))
    }
}

/// The full request path. Inside `.nest("/api", …)` `parts.uri` has the prefix stripped;
/// axum keeps the original in `OriginalUri`.
fn request_path(parts: &Parts) -> &str {
    parts
        .extensions
        .get::<OriginalUri>()
        .map(|u| u.0.path())
        .unwrap_or_else(|| parts.uri.path())
}

/// Sign-in, sign-out, `/auth/me` and the verification routes stay open to unverified accounts
/// under the hard gate, so they can verify or leave.
fn gate_exempt(path: &str) -> bool {
    path.starts_with("/api/auth/") || path.starts_with("/auth/")
}

/// Authenticated user that must be an admin.
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_admin {
            return Err(Error::Forbidden.into());
        }
        Ok(AdminUser(user))
    }
}

/// Sent by the web app on every request. A page on another site cannot add a custom header to
/// a credentialed request without a CORS preflight, which the API never grants with
/// credentials, so a session cookie alone never authorises a write.
pub const CLIENT_HEADER: &str = "x-linkdean-client";

/// The session token from `Authorization: Bearer` (API clients) or the `linkdean_session`
/// cookie (the browser). A cookie only counts on a write when the client header is present.
pub fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(h) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = h.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return Some(rest.to_string());
            }
        }
    }
    if !parts.method.is_safe() && !parts.headers.contains_key(CLIENT_HEADER) {
        return None;
    }
    parts
        .headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|c| {
            c.split(';').find_map(|part| {
                let part = part.trim();
                part.strip_prefix("linkdean_session=")
                    .map(|v| v.to_string())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Request};

    fn parts(method: Method, headers: &[(&str, &str)]) -> Parts {
        let mut req = Request::builder().method(method).uri("/api/bookmarks");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        req.body(()).unwrap().into_parts().0
    }

    #[test]
    fn only_auth_paths_are_exempt_from_the_gate() {
        assert!(gate_exempt("/api/auth/me"));
        assert!(gate_exempt("/api/auth/email/resend"));
        assert!(gate_exempt("/auth/logout"));
        assert!(!gate_exempt("/api/bookmarks"));
        assert!(!gate_exempt("/api/authors"));
        assert!(!gate_exempt("/api/shares/claim"));
    }

    #[test]
    fn the_request_path_prefers_the_original_uri() {
        let mut p = parts(Method::GET, &[]);
        p.uri = "/bookmarks".parse().unwrap();
        assert_eq!(request_path(&p), "/bookmarks");
        p.extensions
            .insert(OriginalUri("/api/bookmarks".parse().unwrap()));
        assert_eq!(request_path(&p), "/api/bookmarks");
    }

    #[test]
    fn a_cookie_reads_without_the_client_header() {
        let p = parts(Method::GET, &[("cookie", "linkdean_session=abc")]);
        assert_eq!(extract_token(&p).as_deref(), Some("abc"));
    }

    #[test]
    fn a_cookie_alone_cannot_write() {
        let p = parts(Method::POST, &[("cookie", "linkdean_session=abc")]);
        assert_eq!(extract_token(&p), None);
        let p = parts(
            Method::DELETE,
            &[
                ("cookie", "a=1; linkdean_session=abc"),
                (CLIENT_HEADER, "web"),
            ],
        );
        assert_eq!(extract_token(&p).as_deref(), Some("abc"));
    }

    #[test]
    fn a_bearer_token_writes_without_the_client_header() {
        let p = parts(Method::PATCH, &[("authorization", "Bearer key")]);
        assert_eq!(extract_token(&p).as_deref(), Some("key"));
    }
}
