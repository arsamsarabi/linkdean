//! In-memory limits on credential guessing and email floods. They reset when the API restarts.

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
/// Abuse reports one user may file per window, so nobody floods the admin queue.
const REPORT_USER: u32 = 10;
/// Reset link requests for one address, so nobody floods an inbox.
const RESET_REQUEST_ACCOUNT: u32 = 3;
const RESET_REQUEST_ADDRESS: u32 = 10;
/// Attempts to use a reset link, from one address.
const RESET_ATTEMPT_ADDRESS: u32 = 10;
/// Verification and email change emails for one account, so nobody floods an inbox.
const VERIFY_RESEND_ACCOUNT: u32 = 3;
const VERIFY_RESEND_ADDRESS: u32 = 10;
/// Attempts to use a verification or email change link, from one address.
const VERIFY_ATTEMPT_ADDRESS: u32 = 10;

/// Invite emails one owner may send per hour, so a compromised account cannot use the instance
/// to spam addresses. Counted in memory, in its own limiter with an hour-long window.
pub const INVITE_EMAIL_WINDOW: Duration = Duration::from_secs(60 * 60);
const INVITE_EMAIL_OWNER: u32 = 20;

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

/// Every request counts, known address or not, so the limit says nothing about accounts.
pub fn reset_request_limits(ip: IpAddr, email: &str) -> [Limit; 2] {
    let email = email.trim().to_ascii_lowercase();
    [
        (
            format!("reset-request-account:{email}"),
            RESET_REQUEST_ACCOUNT,
        ),
        (format!("reset-request-address:{ip}"), RESET_REQUEST_ADDRESS),
    ]
}

pub fn reset_attempt_limit(ip: IpAddr) -> [Limit; 1] {
    [(format!("reset-attempt:{ip}"), RESET_ATTEMPT_ADDRESS)]
}

/// Verification and email change emails, per account and per address.
pub fn verify_resend_limits(user_id: uuid::Uuid, ip: IpAddr) -> [Limit; 2] {
    [
        (
            format!("verify-resend-account:{user_id}"),
            VERIFY_RESEND_ACCOUNT,
        ),
        (format!("verify-resend-address:{ip}"), VERIFY_RESEND_ADDRESS),
    ]
}

pub fn verify_attempt_limit(ip: IpAddr) -> [Limit; 1] {
    [(format!("verify-attempt:{ip}"), VERIFY_ATTEMPT_ADDRESS)]
}

/// Share invite emails sent by one owner, in `AppState::invite_limiter`.
pub fn invite_email_limit(owner_id: uuid::Uuid) -> [Limit; 1] {
    [(format!("invite-email:{owner_id}"), INVITE_EMAIL_OWNER)]
}

pub fn report_limit(user_id: uuid::Uuid) -> [Limit; 1] {
    [(format!("report:{user_id}"), REPORT_USER)]
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
    fn reset_request_keys_ignore_email_case_and_differ_per_email_and_address() {
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        let other: IpAddr = "203.0.113.10".parse().unwrap();
        assert_eq!(
            reset_request_limits(ip, " Bob@X.com "),
            reset_request_limits(ip, "bob@x.com")
        );
        let [bob_account, bob_address] = reset_request_limits(ip, "bob@x.com");
        let [amy_account, amy_address] = reset_request_limits(ip, "amy@x.com");
        assert_ne!(bob_account, amy_account);
        assert_eq!(bob_address, amy_address, "one counter per address");
        let [elsewhere_account, elsewhere_address] = reset_request_limits(other, "bob@x.com");
        assert_eq!(bob_account, elsewhere_account, "one counter per email");
        assert_ne!(bob_address, elsewhere_address);
        assert_eq!(bob_account.1, 3);
        assert_eq!(bob_address.1, 10);
    }

    #[test]
    fn reset_attempt_key_differs_per_address() {
        let a: IpAddr = "203.0.113.9".parse().unwrap();
        let b: IpAddr = "203.0.113.10".parse().unwrap();
        assert_ne!(reset_attempt_limit(a), reset_attempt_limit(b));
        assert_eq!(reset_attempt_limit(a)[0].1, 10);
    }

    #[test]
    fn verify_resend_keys_differ_per_account_and_address() {
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        let other: IpAddr = "203.0.113.10".parse().unwrap();
        let (bob, amy) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let [bob_account, bob_address] = verify_resend_limits(bob, ip);
        let [amy_account, amy_address] = verify_resend_limits(amy, ip);
        assert_ne!(bob_account, amy_account);
        assert_eq!(bob_address, amy_address, "one counter per address");
        let [elsewhere_account, elsewhere_address] = verify_resend_limits(bob, other);
        assert_eq!(bob_account, elsewhere_account, "one counter per account");
        assert_ne!(bob_address, elsewhere_address);
        assert_eq!(bob_account.1, 3);
        assert_eq!(bob_address.1, 10);
        let [reset_account, _] = reset_request_limits(ip, "bob@x.com");
        assert_ne!(bob_account.0, reset_account.0, "apart from reset requests");
    }

    #[test]
    fn verify_attempt_key_differs_per_address_and_from_reset() {
        let a: IpAddr = "203.0.113.9".parse().unwrap();
        let b: IpAddr = "203.0.113.10".parse().unwrap();
        assert_ne!(verify_attempt_limit(a), verify_attempt_limit(b));
        assert_ne!(verify_attempt_limit(a)[0].0, reset_attempt_limit(a)[0].0);
        assert_eq!(verify_attempt_limit(a)[0].1, 10);
    }

    #[test]
    fn the_fourth_verify_email_for_one_account_is_refused() {
        let limiter = Limiter::new(WINDOW);
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        let user = uuid::Uuid::new_v4();
        for _ in 0..3 {
            let limits = verify_resend_limits(user, ip);
            assert!(limiter.check(&limits).is_ok());
            limiter.record(&limits);
        }
        assert!(limiter.check(&verify_resend_limits(user, ip)).is_err());
        assert!(limiter
            .check(&verify_resend_limits(uuid::Uuid::new_v4(), ip))
            .is_ok());
    }

    #[test]
    fn the_fourth_reset_request_for_one_email_is_refused() {
        let limiter = Limiter::new(WINDOW);
        let ip: IpAddr = "203.0.113.9".parse().unwrap();
        for _ in 0..3 {
            let limits = reset_request_limits(ip, "bob@x.com");
            assert!(limiter.check(&limits).is_ok());
            limiter.record(&limits);
        }
        assert!(limiter
            .check(&reset_request_limits(ip, "BOB@x.com"))
            .is_err());
        assert!(limiter
            .check(&reset_request_limits(ip, "amy@x.com"))
            .is_ok());
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
