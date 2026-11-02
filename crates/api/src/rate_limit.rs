//! In-memory limits on credential guessing. They reset when the API restarts.

use axum::http::HeaderMap;
use linkdean_core::{Error, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const WINDOW: Duration = Duration::from_secs(15 * 60);
/// Stops random keys growing the table without limit. Past this, new keys go uncounted.
const MAX_KEYS: usize = 100_000;

/// Failed logins from one address against one account.
const LOGIN_PAIR: u32 = 10;
/// Failed logins against one account, from anywhere. Higher than the pair, so a stranger
/// has to work to lock someone out.
const LOGIN_ACCOUNT: u32 = 50;
const LOGIN_ADDRESS: u32 = 50;
const REGISTER_ADDRESS: u32 = 10;
const VAULT_USER: u32 = 10;

/// A counter and the attempts it allows per window.
pub type Limit = (String, u32);

pub fn login_limits(ip: IpAddr, email: &str) -> [Limit; 3] {
    let email = email.trim().to_ascii_lowercase();
    [
        (format!("login:{ip}:{email}"), LOGIN_PAIR),
        (format!("login-account:{email}"), LOGIN_ACCOUNT),
        (format!("login-address:{ip}"), LOGIN_ADDRESS),
    ]
}

pub fn register_limit(ip: IpAddr) -> [Limit; 1] {
    [(format!("register:{ip}"), REGISTER_ADDRESS)]
}

pub fn vault_limit(user_id: uuid::Uuid) -> [Limit; 1] {
    [(format!("vault:{user_id}"), VAULT_USER)]
}

/// True for the errors a wrong password, passphrase or recovery key produces.
pub fn is_wrong_secret(error: &Error) -> bool {
    matches!(error, Error::Unauthorized | Error::VaultUnlockFailed)
}

#[derive(Clone, Copy)]
struct Window {
    started: Instant,
    count: u32,
}

pub struct Limiter {
    window: Duration,
    keys: Mutex<HashMap<String, Window>>,
}

impl Limiter {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            keys: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Window>> {
        self.keys.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Refuse when any limit is used up.
    pub fn check(&self, limits: &[Limit]) -> Result<()> {
        let keys = self.lock();
        let now = Instant::now();
        for (key, max) in limits {
            if let Some(w) = keys.get(key) {
                if now.duration_since(w.started) < self.window && w.count >= *max {
                    return Err(Error::TooManyRequests);
                }
            }
        }
        Ok(())
    }

    pub fn record(&self, limits: &[Limit]) {
        let mut keys = self.lock();
        let now = Instant::now();
        if keys.len() >= MAX_KEYS {
            let window = self.window;
            keys.retain(|_, w| now.duration_since(w.started) < window);
        }
        for (key, _) in limits {
            if !keys.contains_key(key) && keys.len() >= MAX_KEYS {
                continue;
            }
            let w = keys.entry(key.clone()).or_insert(Window {
                started: now,
                count: 0,
            });
            if now.duration_since(w.started) >= self.window {
                *w = Window {
                    started: now,
                    count: 0,
                };
            }
            w.count += 1;
        }
    }

    /// Count a wrong secret, or clear the first counter after a success.
    pub fn settle<T>(&self, limits: &[Limit], result: Result<T>) -> Result<T> {
        match &result {
            Ok(_) => {
                if let Some(first) = limits.first() {
                    self.clear(first);
                }
            }
            Err(e) if is_wrong_secret(e) => self.record(limits),
            Err(_) => {}
        }
        result
    }

    /// Forget a counter after a success, so typos do not add up across a day.
    pub fn clear(&self, limit: &Limit) {
        self.lock().remove(&limit.0);
    }
}

/// The caller's address. `X-Forwarded-For` can be forged, so it only counts behind a trusted
/// proxy, and only its last entry, which that proxy wrote.
pub fn client_ip(headers: &HeaderMap, peer: IpAddr, trust_forwarded_for: bool) -> IpAddr {
    if !trust_forwarded_for {
        return peer;
    }
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(peer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn limit(key: &str, max: u32) -> Vec<Limit> {
        vec![(key.to_string(), max)]
    }

    #[test]
    fn refuses_after_the_limit_and_keeps_keys_apart() {
        let limiter = Limiter::new(WINDOW);
        let a = limit("a", 2);
        limiter.record(&a);
        assert!(limiter.check(&a).is_ok());
        limiter.record(&a);
        assert!(matches!(limiter.check(&a), Err(Error::TooManyRequests)));
        assert!(limiter.check(&limit("b", 2)).is_ok());
    }

    #[test]
    fn any_exhausted_limit_refuses() {
        let limiter = Limiter::new(WINDOW);
        limiter.record(&limit("shared", 1));
        let both = vec![("fresh".to_string(), 5), ("shared".to_string(), 1)];
        assert!(limiter.check(&both).is_err());
    }

    #[test]
    fn the_window_expires() {
        let limiter = Limiter::new(Duration::from_millis(5));
        let a = limit("a", 1);
        limiter.record(&a);
        assert!(limiter.check(&a).is_err());
        std::thread::sleep(Duration::from_millis(10));
        assert!(limiter.check(&a).is_ok());
        limiter.record(&a);
        assert!(limiter.check(&a).is_err(), "a new window counts again");
    }

    #[test]
    fn success_clears_only_the_given_counter() {
        let limiter = Limiter::new(WINDOW);
        let pair = limit("pair", 1);
        let account = limit("account", 1);
        limiter.record(&pair);
        limiter.record(&account);
        limiter.clear(&pair[0]);
        assert!(limiter.check(&pair).is_ok());
        assert!(limiter.check(&account).is_err());
    }

    #[test]
    fn settle_counts_wrong_secrets_only() {
        let limiter = Limiter::new(WINDOW);
        let a = limit("a", 1);
        let _ = limiter.settle::<()>(&a, Err(Error::BadRequest("nope".into())));
        assert!(limiter.check(&a).is_ok());
        let _ = limiter.settle::<()>(&a, Err(Error::VaultUnlockFailed));
        assert!(limiter.check(&a).is_err());
        let _ = limiter.settle(&a, Ok(()));
        assert!(limiter.check(&a).is_ok());
    }

    #[test]
    fn login_keys_ignore_email_case_and_padding() {
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        assert_eq!(
            login_limits(ip, " Bob@X.com "),
            login_limits(ip, "bob@x.com")
        );
    }

    #[test]
    fn forwarded_for_only_counts_when_trusted_and_uses_the_last_entry() {
        let peer: IpAddr = "10.0.0.1".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("1.1.1.1, 203.0.113.9"),
        );
        assert_eq!(client_ip(&headers, peer, false), peer);
        assert_eq!(
            client_ip(&headers, peer, true),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
        assert_eq!(client_ip(&HeaderMap::new(), peer, true), peer);
    }
}
