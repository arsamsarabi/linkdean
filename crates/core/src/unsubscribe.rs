//! Signed one-click unsubscribe links for notification email (RFC 8058).
//!
//! A link names a user and a scope (one notification kind, or the daily digest) and carries an
//! HMAC-SHA256 of both under the server's secret, so it works signed out and cannot be forged
//! for someone else. It never expires; changing `SESSION_SECRET` voids every link.

use crate::NotificationKind;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

/// What one link turns off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsubscribeScope {
    /// Email for one notification kind.
    Kind(NotificationKind),
    /// The daily digest: every kind the user gets that way.
    Digest,
}

impl UnsubscribeScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kind(kind) => kind.as_str(),
            Self::Digest => "digest",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        if s == "digest" {
            return Some(Self::Digest);
        }
        NotificationKind::parse(s).ok().map(Self::Kind)
    }

    /// What the unsubscribe page and the email footer call it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Kind(kind) => kind.label(),
            Self::Digest => "Daily digest",
        }
    }
}

fn mac(secret: &str, user_id: Uuid, scope: UnsubscribeScope) -> Hmac<Sha256> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC takes a key of any length");
    mac.update(b"linkdean-unsubscribe-v1\0");
    mac.update(user_id.as_bytes());
    mac.update(b"\0");
    mac.update(scope.as_str().as_bytes());
    mac
}

/// The token for `user_id` and `scope`.
pub fn token(secret: &str, user_id: Uuid, scope: UnsubscribeScope) -> String {
    URL_SAFE_NO_PAD.encode(mac(secret, user_id, scope).finalize().into_bytes())
}

/// Whether `token` was made by [`token`] for this user and scope. Constant time.
pub fn verify(secret: &str, user_id: Uuid, scope: UnsubscribeScope, token: &str) -> bool {
    let Ok(bytes) = URL_SAFE_NO_PAD.decode(token.trim()) else {
        return false;
    };
    mac(secret, user_id, scope).verify_slice(&bytes).is_ok()
}

/// The query string (`u=…&s=…&t=…`) both the web page and the one-click POST take.
pub fn query(secret: &str, user_id: Uuid, scope: UnsubscribeScope) -> String {
    format!(
        "u={user_id}&s={}&t={}",
        scope.as_str(),
        token(secret, user_id, scope)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "a-test-secret-of-some-length";

    #[test]
    fn a_token_verifies_only_for_its_user_scope_and_secret() {
        let user = Uuid::new_v4();
        let scope = UnsubscribeScope::Kind(NotificationKind::SocialLike);
        let t = token(SECRET, user, scope);
        assert_eq!(t.len(), 43, "32 bytes, base64url without padding");
        assert!(verify(SECRET, user, scope, &t));
        assert!(!verify(SECRET, Uuid::new_v4(), scope, &t));
        assert!(!verify(SECRET, user, UnsubscribeScope::Digest, &t));
        assert!(!verify("another-secret-entirely", user, scope, &t));
        assert!(!verify(SECRET, user, scope, &t[..42]));
        assert!(!verify(SECRET, user, scope, "not base64 at all!"));
        assert!(!verify(SECRET, user, scope, ""));
    }

    #[test]
    fn the_same_inputs_give_the_same_token() {
        let user = Uuid::nil();
        assert_eq!(
            token(SECRET, user, UnsubscribeScope::Digest),
            token(SECRET, user, UnsubscribeScope::Digest)
        );
    }

    #[test]
    fn scopes_round_trip() {
        for kind in NotificationKind::ALL {
            let scope = UnsubscribeScope::Kind(kind);
            assert_eq!(UnsubscribeScope::parse(scope.as_str()), Some(scope));
        }
        assert_eq!(
            UnsubscribeScope::parse("digest"),
            Some(UnsubscribeScope::Digest)
        );
        assert_eq!(UnsubscribeScope::parse("everything"), None);
    }

    #[test]
    fn the_query_carries_user_scope_and_token() {
        let user = Uuid::nil();
        let q = query(SECRET, user, UnsubscribeScope::Digest);
        assert!(q.starts_with(&format!("u={user}&s=digest&t=")));
        assert!(q.ends_with(&token(SECRET, user, UnsubscribeScope::Digest)));
    }
}
