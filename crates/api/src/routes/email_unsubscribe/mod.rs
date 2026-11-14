//! One-click unsubscribe from notification email, signed out (RFC 8058).
//!
//! - `GET /api/email/unsubscribe?u=&s=&t=` checks a link and says what it turns off. It changes
//!   nothing, because link scanners open every URL in an email.
//! - `POST` with the same query turns it off. Mail apps send this when the user presses their
//!   Unsubscribe button (`List-Unsubscribe-Post: List-Unsubscribe=One-Click`, any body), and the
//!   web page's button sends it too.
//!
//! A bad, forged or stale link answers 404, the same as a missing user.

use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::unsubscribe::{self, UnsubscribeScope};
use linkdean_core::Error;
use linkdean_db as db;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/email/unsubscribe", get(check).post(apply))
}

#[derive(Debug, Deserialize)]
struct UnsubscribeLink {
    u: Option<String>,
    s: Option<String>,
    t: Option<String>,
}

/// The user and scope a link names, when its signature holds.
fn verified(secret: &str, link: &UnsubscribeLink) -> Result<(Uuid, UnsubscribeScope), Error> {
    let user = link.u.as_deref().and_then(|u| u.parse::<Uuid>().ok());
    let scope = link.s.as_deref().and_then(UnsubscribeScope::parse);
    match (user, scope, link.t.as_deref()) {
        (Some(user), Some(scope), Some(token))
            if unsubscribe::verify(secret, user, scope, token) =>
        {
            Ok((user, scope))
        }
        _ => Err(Error::NotFound),
    }
}

fn describe(scope: UnsubscribeScope) -> Value {
    json!({ "scope": scope.as_str(), "label": scope.label() })
}

async fn check(
    State(state): State<AppState>,
    Query(link): Query<UnsubscribeLink>,
) -> ApiResult<Json<Value>> {
    let (_, scope) = verified(&state.cfg.session_secret, &link)?;
    Ok(Json(describe(scope)))
}

async fn apply(
    State(state): State<AppState>,
    Query(link): Query<UnsubscribeLink>,
) -> ApiResult<Json<Value>> {
    let (user, scope) = verified(&state.cfg.session_secret, &link)?;
    let done = db::unsubscribe(&state.pool, user, scope).await?;
    tracing::info!(user_id = %user, scope = scope.as_str(), kinds = done.kinds.len(), "unsubscribed from notification email");
    let mut out = describe(scope);
    out["unsubscribed"] = json!(done.kinds);
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkdean_core::NotificationKind;

    const SECRET: &str = "a-test-secret-of-some-length";

    fn link(u: Option<&str>, s: Option<&str>, t: Option<&str>) -> UnsubscribeLink {
        UnsubscribeLink {
            u: u.map(Into::into),
            s: s.map(Into::into),
            t: t.map(Into::into),
        }
    }

    #[test]
    fn only_a_signed_link_names_its_user_and_scope() {
        let user = Uuid::new_v4();
        let scope = UnsubscribeScope::Kind(NotificationKind::SocialComment);
        let token = unsubscribe::token(SECRET, user, scope);
        let id = user.to_string();
        assert_eq!(
            verified(
                SECRET,
                &link(Some(&id), Some("social_comment"), Some(&token))
            )
            .unwrap(),
            (user, scope)
        );
        for bad in [
            link(Some(&id), Some("social_like"), Some(&token)),
            link(
                Some(&Uuid::new_v4().to_string()),
                Some("social_comment"),
                Some(&token),
            ),
            link(Some(&id), Some("social_comment"), Some("forged")),
            link(Some("not-a-uuid"), Some("social_comment"), Some(&token)),
            link(Some(&id), Some("everything"), Some(&token)),
            link(Some(&id), Some("social_comment"), None),
            link(None, None, None),
        ] {
            assert!(
                matches!(verified(SECRET, &bad), Err(Error::NotFound)),
                "{bad:?}"
            );
        }
    }
}
