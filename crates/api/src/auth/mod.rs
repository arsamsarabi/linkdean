use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
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
        let token = extract_token(parts).ok_or(Error::Unauthorized)?;
        let user = db::user_from_token(&state.pool, &token)
            .await?
            .ok_or(Error::Unauthorized)?;
        Ok(AuthUser(user))
    }
}

pub fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(h) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = h.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return Some(rest.to_string());
            }
        }
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
