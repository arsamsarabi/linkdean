//! Unlocked vault keys, held in memory for one login session each.
//!
//! A key unlocked in one browser session is not usable from another session or from an API
//! key. It goes when the session logs out or ends, when the user's password changes or they
//! are banned, and after the user's vault auto-lock time passes with no request using it.

use crate::auth::extract_token;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use linkdean_core::crypto::hash_token;
use linkdean_core::{Error, Result, VaultKeyring};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// The login session a request came from, as the hash stored in `sessions.token_hash`.
/// API keys and requests with no token have none, so they never see vault content.
#[derive(Debug, Clone, Default)]
pub struct VaultSession(Option<String>);

impl VaultSession {
    pub fn from_token(token: &str) -> Self {
        if token.starts_with("ldk_") {
            Self(None)
        } else {
            Self(Some(hash_token(token)))
        }
    }

    pub fn is_session(&self) -> bool {
        self.0.is_some()
    }

    /// The login session's token hash; `None` for API keys and anonymous requests.
    pub fn token_hash(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl FromRequestParts<AppState> for VaultSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        Ok(extract_token(parts)
            .map(|t| Self::from_token(&t))
            .unwrap_or_default())
    }
}

/// How long unlocked vaults stay open with no request using them. `0` means until the
/// session ends.
pub fn idle_limit(auto_lock_minutes: i32) -> Option<Duration> {
    (auto_lock_minutes > 0).then(|| Duration::from_secs(auto_lock_minutes as u64 * 60))
}

struct Unlocked {
    user_id: Uuid,
    ring: VaultKeyring,
    last_used: Instant,
    idle: Option<Duration>,
}

impl Unlocked {
    fn expired(&self, now: Instant) -> bool {
        self.idle
            .is_some_and(|idle| now.saturating_duration_since(self.last_used) >= idle)
    }
}

#[derive(Default)]
pub struct VaultKeyStore {
    sessions: Mutex<HashMap<String, Unlocked>>,
}

impl VaultKeyStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Unlocked>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The vaults this session has unlocked, and marks them used now.
    pub fn keyring(&self, session: &VaultSession, user_id: Uuid) -> VaultKeyring {
        self.keyring_at(session, user_id, Instant::now())
    }

    fn keyring_at(&self, session: &VaultSession, user_id: Uuid, now: Instant) -> VaultKeyring {
        let Some(hash) = &session.0 else {
            return VaultKeyring::default();
        };
        let mut sessions = self.lock();
        match sessions.get_mut(hash) {
            Some(u) if u.user_id == user_id && !u.expired(now) => {
                u.last_used = now;
                u.ring.clone()
            }
            Some(_) => {
                sessions.remove(hash);
                VaultKeyring::default()
            }
            None => VaultKeyring::default(),
        }
    }

    /// Unlock `vault_id` for this session. API keys may not.
    pub fn insert(
        &self,
        session: &VaultSession,
        user_id: Uuid,
        vault_id: Uuid,
        key: [u8; 32],
        idle: Option<Duration>,
    ) -> Result<()> {
        self.insert_at(session, user_id, vault_id, key, idle, Instant::now())
    }

    fn insert_at(
        &self,
        session: &VaultSession,
        user_id: Uuid,
        vault_id: Uuid,
        key: [u8; 32],
        idle: Option<Duration>,
        now: Instant,
    ) -> Result<()> {
        let Some(hash) = &session.0 else {
            return Err(Error::Forbidden);
        };
        let mut sessions = self.lock();
        let fresh = || Unlocked {
            user_id,
            ring: VaultKeyring::default(),
            last_used: now,
            idle,
        };
        let entry = sessions.entry(hash.clone()).or_insert_with(fresh);
        if entry.user_id != user_id || entry.expired(now) {
            *entry = fresh();
        }
        entry.ring.insert(vault_id, key);
        entry.last_used = now;
        entry.idle = idle;
        Ok(())
    }

    /// Lock one vault in every session of the user.
    pub fn remove_vault(&self, user_id: Uuid, vault_id: Uuid) {
        let mut sessions = self.lock();
        for u in sessions.values_mut().filter(|u| u.user_id == user_id) {
            let mut next = VaultKeyring::default();
            for id in u.ring.vault_ids() {
                if id != vault_id {
                    if let Some(key) = u.ring.get(id) {
                        next.insert(id, *key);
                    }
                }
            }
            u.ring = next;
        }
        sessions.retain(|_, u| !u.ring.is_empty());
    }

    /// Lock every vault in every session of the user.
    pub fn clear_user(&self, user_id: Uuid) {
        self.lock().retain(|_, u| u.user_id != user_id);
    }

    pub fn clear_session(&self, session: &VaultSession) {
        if let Some(hash) = &session.0 {
            self.lock().remove(hash);
        }
    }

    /// Apply a changed auto-lock time to the user's open sessions.
    pub fn set_idle(&self, user_id: Uuid, idle: Option<Duration>) {
        for u in self.lock().values_mut().filter(|u| u.user_id == user_id) {
            u.idle = idle;
        }
    }

    /// Session hashes holding keys, for checking which sessions still exist.
    pub fn session_hashes(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    /// Drop keys that sat idle past their limit, and keys of sessions not in `live`.
    pub fn sweep(&self, live: &HashSet<String>) -> usize {
        self.sweep_at(live, Instant::now())
    }

    fn sweep_at(&self, live: &HashSet<String>, now: Instant) -> usize {
        let mut sessions = self.lock();
        let before = sessions.len();
        sessions.retain(|hash, u| live.contains(hash) && !u.expired(now));
        before - sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7; 32];

    fn session(token: &str) -> VaultSession {
        VaultSession::from_token(token)
    }

    #[test]
    fn a_key_belongs_to_the_session_that_unlocked_it() {
        let store = VaultKeyStore::default();
        let (user, vault) = (Uuid::new_v4(), Uuid::new_v4());
        store
            .insert(&session("laptop"), user, vault, KEY, None)
            .unwrap();

        assert_eq!(
            store.keyring(&session("laptop"), user).get(vault),
            Some(&KEY)
        );
        assert!(store.keyring(&session("phone"), user).is_empty());
        assert!(store.keyring(&session("laptop"), Uuid::new_v4()).is_empty());
    }

    #[test]
    fn api_keys_never_hold_or_see_vault_keys() {
        let store = VaultKeyStore::default();
        let user = Uuid::new_v4();
        let api = session("ldk_abc");
        assert!(!api.is_session());
        assert!(matches!(
            store.insert(&api, user, Uuid::new_v4(), KEY, None),
            Err(Error::Forbidden)
        ));
        assert!(store.keyring(&api, user).is_empty());
        assert!(store.keyring(&VaultSession::default(), user).is_empty());
    }

    #[test]
    fn keys_lock_after_the_idle_limit_unless_used() {
        let store = VaultKeyStore::default();
        let (user, vault) = (Uuid::new_v4(), Uuid::new_v4());
        let s = session("tab");
        let start = Instant::now();
        let idle = idle_limit(5);
        store.insert_at(&s, user, vault, KEY, idle, start).unwrap();

        let used = start + Duration::from_secs(4 * 60);
        assert!(!store.keyring_at(&s, user, used).is_empty());
        // Use moved the clock, so the limit counts from there.
        let later = used + Duration::from_secs(4 * 60);
        assert!(!store.keyring_at(&s, user, later).is_empty());
        let idle_too_long = later + Duration::from_secs(5 * 60);
        assert!(store.keyring_at(&s, user, idle_too_long).is_empty());
        assert!(
            store.session_hashes().is_empty(),
            "expired keys are dropped"
        );
    }

    #[test]
    fn zero_minutes_keeps_keys_for_the_session() {
        assert_eq!(idle_limit(0), None);
        assert_eq!(idle_limit(-3), None);
        let store = VaultKeyStore::default();
        let (user, vault) = (Uuid::new_v4(), Uuid::new_v4());
        let s = session("tab");
        let start = Instant::now();
        store.insert_at(&s, user, vault, KEY, None, start).unwrap();
        let next_year = start + Duration::from_secs(365 * 24 * 3600);
        assert!(!store.keyring_at(&s, user, next_year).is_empty());
    }

    #[test]
    fn sweep_drops_ended_sessions_and_idle_keys() {
        let store = VaultKeyStore::default();
        let user = Uuid::new_v4();
        let start = Instant::now();
        for (token, idle) in [("live", None), ("ended", None), ("idle", idle_limit(1))] {
            store
                .insert_at(&session(token), user, Uuid::new_v4(), KEY, idle, start)
                .unwrap();
        }
        let live: HashSet<String> = ["live", "idle"].iter().map(|t| hash_token(t)).collect();
        let removed = store.sweep_at(&live, start + Duration::from_secs(120));
        assert_eq!(removed, 2);
        assert_eq!(store.session_hashes(), vec![hash_token("live")]);
    }

    #[test]
    fn locking_reaches_every_session_of_the_user_only() {
        let store = VaultKeyStore::default();
        let (user, other, a, b) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        for token in ["one", "two"] {
            store.insert(&session(token), user, a, KEY, None).unwrap();
            store.insert(&session(token), user, b, KEY, None).unwrap();
        }
        store
            .insert(&session("theirs"), other, a, KEY, None)
            .unwrap();

        store.remove_vault(user, a);
        for token in ["one", "two"] {
            let ring = store.keyring(&session(token), user);
            assert!(ring.get(a).is_none());
            assert!(ring.get(b).is_some());
        }
        assert!(store.keyring(&session("theirs"), other).get(a).is_some());

        store.clear_user(user);
        assert!(store.keyring(&session("one"), user).is_empty());
        assert!(!store.keyring(&session("theirs"), other).is_empty());

        store.clear_session(&session("theirs"));
        assert!(store.keyring(&session("theirs"), other).is_empty());
    }
}
