#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub meili_url: String,
    pub meili_key: String,
    pub app_url: String,
    /// Reserved for future cookie/HMAC signing. Sessions today use hashed opaque tokens in Postgres.
    /// Validated at startup (min length); not yet applied to token hashing.
    #[allow(dead_code)]
    pub session_secret: String,
    pub session_days: i64,
    pub avatar_dir: String,
    pub preview_dir: String,
    pub browser_ws_url: Option<String>,
    /// Where to serve the forward proxy Chrome uses for all its traffic. Off when unset.
    pub browser_proxy_bind: Option<std::net::SocketAddr>,
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    /// Empty = reflect `APP_URL` only. Use `*` for any-origin (dev/homelab convenience).
    pub cors_origins: Vec<String>,
    /// Behind a reverse proxy, read the client address from `X-Forwarded-For`.
    pub trust_forwarded_for: bool,
    /// When set, send `Strict-Transport-Security` with this max-age in seconds.
    /// Leave unset on plain HTTP so browsers are not stuck on HSTS for localhost.
    pub hsts_max_age: Option<u64>,
    /// Bearer token for `GET /api/metrics`. Unset turns the route off.
    pub metrics_token: Option<String>,
    /// When set, the first account (the admin) must present it, so whoever finds a fresh
    /// install first cannot claim it.
    pub bootstrap_token: Option<String>,
    /// SMTP settings. `None` keeps mail off and every feature that needs it on its fallback.
    pub mail: Option<linkdean_mail::MailConfig>,
    /// Cloudflare Turnstile site key (public). Both keys must be set or both unset.
    pub turnstile_site_key: Option<String>,
    /// Cloudflare Turnstile secret (server only).
    pub turnstile_secret_key: Option<String>,
    /// Origin of a GoatCounter instance (`https://stats.example.com`) that counts page views
    /// in the web app. Unset keeps analytics off. Only route names are sent, never ids or queries.
    pub goatcounter_url: Option<String>,
    /// When true and mail is on, require verified email even if the platform setting is off.
    pub require_verified_email: bool,
    /// When true, refuse registration / email-change to known disposable domains.
    pub block_disposable_emails: bool,
    /// Development-only routes such as `/api/dev/emails`. On with `ALLOW_DEFAULT_SECRETS`.
    pub dev_routes: bool,
    /// Experimental: `GET /api/agent-skill` and its UI. Off unless `EXPERIMENTAL_AGENT_SKILLS`.
    pub experimental_agent_skills: bool,
    pub link_check: linkdean_worker::LinkCheckOpts,
}

/// `GOATCOUNTER_URL` must be a bare origin: https, or http on this machine, with no path,
/// query or credentials. It is added to the CSP, so anything looser is refused at startup.
pub fn parse_goatcounter_url(raw: &str) -> anyhow::Result<String> {
    let bad =
        || anyhow::anyhow!("GOATCOUNTER_URL must be an origin such as https://stats.example.com");
    let url = url::Url::parse(raw).map_err(|_| bad())?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    let scheme_ok = url.scheme() == "https" || (url.scheme() == "http" && local);
    if !scheme_ok
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(bad());
    }
    Ok(url.origin().ascii_serialization())
}

/// Values published in this repository, so nobody should run on them in production.
const KNOWN_SECRETS: [&str; 4] = [
    "change-me-to-a-long-random-string",
    "replace-with-long-random-string",
    "dev_session_secret_change_me",
    "dev_master_key_change_me",
];
const KNOWN_DATABASE_LOGIN: &str = "linkdean:linkdean@";

/// Refuse to start on a published secret unless development explicitly allows it.
fn refuse_known_secrets(
    session_secret: &str,
    meili_key: Option<&str>,
    database_url: &str,
) -> anyhow::Result<()> {
    if KNOWN_SECRETS.contains(&session_secret) {
        anyhow::bail!("SESSION_SECRET is a published default; generate your own");
    }
    if meili_key.is_some_and(|key| KNOWN_SECRETS.contains(&key)) {
        anyhow::bail!("MEILI_MASTER_KEY is a published default; generate your own");
    }
    if database_url.contains(KNOWN_DATABASE_LOGIN) {
        anyhow::bail!("DATABASE_URL uses the published default password; set your own");
    }
    Ok(())
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        if !env_bool("ALLOW_DEFAULT_SECRETS", false) {
            refuse_known_secrets(
                &required("SESSION_SECRET")?,
                std::env::var("MEILI_MASTER_KEY").ok().as_deref(),
                &required("DATABASE_URL")?,
            )?;
        }
        let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:8080".into());
        let cors_origins = match std::env::var("CORS_ORIGINS") {
            Ok(s) if s.trim() == "*" => vec!["*".into()],
            Ok(s) if !s.trim().is_empty() => s
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect(),
            _ => vec![app_url.clone()],
        };
        Ok(Self {
            bind: std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            database_url: required("DATABASE_URL")?,
            meili_url: std::env::var("MEILI_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:7700".into()),
            meili_key: std::env::var("MEILI_MASTER_KEY")
                .unwrap_or_else(|_| "dev_master_key_change_me".into()),
            app_url,
            session_secret: {
                let s = required("SESSION_SECRET")?;
                if s.len() < 16 {
                    anyhow::bail!("SESSION_SECRET must be at least 16 characters");
                }
                s
            },
            session_days: std::env::var("SESSION_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            avatar_dir: std::env::var("AVATAR_DIR").unwrap_or_else(|_| "./data/avatars".into()),
            preview_dir: std::env::var("PREVIEW_DIR").unwrap_or_else(|_| "./data/previews".into()),
            browser_ws_url: std::env::var("BROWSER_WS_URL")
                .ok()
                .filter(|s| !s.is_empty()),
            browser_proxy_bind: match std::env::var("BROWSER_PROXY_BIND") {
                Ok(s) if !s.is_empty() => Some(
                    s.parse()
                        .map_err(|_| anyhow::anyhow!("BROWSER_PROXY_BIND must be address:port"))?,
                ),
                _ => None,
            },
            oidc_issuer: std::env::var("OIDC_ISSUER").ok().filter(|s| !s.is_empty()),
            oidc_client_id: std::env::var("OIDC_CLIENT_ID")
                .ok()
                .filter(|s| !s.is_empty()),
            oidc_client_secret: std::env::var("OIDC_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
            cors_origins,
            trust_forwarded_for: env_bool("TRUST_FORWARDED_FOR", false),
            hsts_max_age: std::env::var("HSTS_MAX_AGE")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse::<u64>()
                        .map_err(|_| anyhow::anyhow!("HSTS_MAX_AGE must be a number of seconds"))
                })
                .transpose()?,
            metrics_token: std::env::var("METRICS_TOKEN")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            bootstrap_token: std::env::var("BOOTSTRAP_TOKEN")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            mail: linkdean_mail::MailConfig::from_vars(|k| std::env::var(k).ok())?,
            turnstile_site_key: None,
            turnstile_secret_key: None,
            goatcounter_url: std::env::var("GOATCOUNTER_URL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(|s| parse_goatcounter_url(&s))
                .transpose()?,
            require_verified_email: env_bool("REQUIRE_VERIFIED_EMAIL", false),
            block_disposable_emails: env_bool("BLOCK_DISPOSABLE_EMAILS", false),
            dev_routes: env_bool("ALLOW_DEFAULT_SECRETS", false),
            experimental_agent_skills: env_bool("EXPERIMENTAL_AGENT_SKILLS", false),
            link_check: linkdean_worker::LinkCheckOpts {
                enabled: env_bool("LINK_CHECK_ENABLED", true),
                tick_secs: env_u64("LINK_CHECK_TICK_SECS", 60),
                batch_size: env_i64("LINK_CHECK_BATCH_SIZE", 40),
                concurrency: env_usize("LINK_CHECK_CONCURRENCY", 2),
                per_host_delay_ms: env_u64("LINK_CHECK_PER_HOST_DELAY_MS", 1_000),
                recheck_days: env_i64("LINK_CHECK_RECHECK_DAYS", 7),
            },
        }
        .with_turnstile(optional_pair("TURNSTILE_SITE_KEY", "TURNSTILE_SECRET_KEY")?))
    }

    fn with_turnstile(mut self, keys: Option<(String, String)>) -> Self {
        if let Some((site, secret)) = keys {
            self.turnstile_site_key = Some(site);
            self.turnstile_secret_key = Some(secret);
        }
        self
    }

    pub fn oidc_enabled(&self) -> bool {
        self.oidc_issuer.is_some()
            && self.oidc_client_id.is_some()
            && self.oidc_client_secret.is_some()
    }

    pub fn turnstile_enabled(&self) -> bool {
        self.turnstile_site_key.is_some() && self.turnstile_secret_key.is_some()
    }

    /// `None` when no bootstrap token is configured, else whether `given` matches it.
    pub fn bootstrap_token_ok(&self, given: Option<&str>) -> Option<bool> {
        self.bootstrap_token.as_deref().map(|expected| {
            given.is_some_and(|g| {
                linkdean_core::crypto::constant_time_eq(g.trim().as_bytes(), expected.as_bytes())
            })
        })
    }

    pub fn cookie_secure(&self) -> bool {
        self.app_url.starts_with("https://")
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("missing env {key}"))
}

/// Both set or both unset. One alone is a config error.
fn optional_pair(a: &str, b: &str) -> anyhow::Result<Option<(String, String)>> {
    let av = std::env::var(a)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let bv = std::env::var(b)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    match (av, bv) {
        (Some(a), Some(b)) => Ok(Some((a, b))),
        (None, None) => Ok(None),
        (Some(_), None) => anyhow::bail!("{a} is set but {b} is missing"),
        (None, Some(_)) => anyhow::bail!("{b} is set but {a} is missing"),
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "postgres://linkdean:a1b2c3@db:5432/linkdean";

    #[test]
    fn goatcounter_url_must_be_a_bare_origin() {
        assert_eq!(
            parse_goatcounter_url("https://stats.example.com").unwrap(),
            "https://stats.example.com"
        );
        assert_eq!(
            parse_goatcounter_url("https://stats.example.com/").unwrap(),
            "https://stats.example.com"
        );
        assert_eq!(
            parse_goatcounter_url("http://localhost:8081").unwrap(),
            "http://localhost:8081"
        );
        for bad in [
            "http://stats.example.com",
            "https://stats.example.com/count",
            "https://stats.example.com?x=1",
            "https://u:p@stats.example.com",
            "https://stats.example.com; script-src *",
            "stats.example.com",
            "javascript:alert(1)",
        ] {
            assert!(parse_goatcounter_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn published_secrets_are_refused() {
        for secret in KNOWN_SECRETS {
            assert!(refuse_known_secrets(secret, None, URL).is_err());
            assert!(refuse_known_secrets("own-secret", Some(secret), URL).is_err());
        }
        let default_login = "postgres://linkdean:linkdean@db:5432/linkdean";
        assert!(refuse_known_secrets("own-secret", None, default_login).is_err());
    }

    #[test]
    fn generated_secrets_pass() {
        assert!(refuse_known_secrets("own-secret", Some("own-key"), URL).is_ok());
    }
}
