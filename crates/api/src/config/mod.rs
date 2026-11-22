#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub meili_url: String,
    pub meili_key: String,
    pub app_url: String,
    /// Signs one-click unsubscribe links in notification email. Sessions use hashed opaque tokens
    /// in Postgres, not this. Validated at startup (min length); changing it voids old links.
    pub session_secret: String,
    pub session_days: i64,
    pub avatar_dir: String,
    pub preview_dir: String,
    /// Archived copies of bookmarks (`ARCHIVE_DIR`).
    pub archive_dir: String,
    /// Archive captures at once, 1 to 8 (`ARCHIVE_CONCURRENCY`). They share Chrome with previews.
    pub archive_concurrency: usize,
    pub browser_ws_url: Option<String>,
    /// Where to serve the forward proxy Chrome uses for all its traffic. Off when unset.
    pub browser_proxy_bind: Option<std::net::SocketAddr>,
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    /// Button label for the generic OIDC provider (`OIDC_NAME`), such as "Authentik".
    pub oidc_name: Option<String>,
    /// Google sign-in: client ID and secret (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`).
    pub google_sso: Option<(String, String)>,
    /// Microsoft Entra ID sign-in for one tenant.
    pub microsoft_sso: Option<MicrosoftSso>,
    /// GitHub sign-in (OAuth app): client ID and secret (`GITHUB_CLIENT_ID`,
    /// `GITHUB_CLIENT_SECRET`).
    pub github_sso: Option<(String, String)>,
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
    /// The operator's Terms of Service and Privacy Policy.
    pub legal: Legal,
    /// When true and mail is on, require verified email even if the platform setting is off.
    pub require_verified_email: bool,
    /// When true, refuse registration / email-change to known disposable domains.
    pub block_disposable_emails: bool,
    /// Development-only routes such as `/api/dev/emails`. On with `ALLOW_DEFAULT_SECRETS`.
    pub dev_routes: bool,
    /// Experimental: `GET /api/agent-skill` and its UI. Off unless `EXPERIMENTAL_AGENT_SKILLS`.
    pub experimental_agent_skills: bool,
    /// Ask GitHub for newer Linkdean releases to show admins. Off with `UPDATE_CHECK=false`.
    pub update_check: bool,
    /// Managed Linkdean service: assign Free/Pro plans and merge them into quotas.
    /// Self-host leaves this off (platform quotas only).
    pub managed_service: bool,
    /// Stripe keys for personal Pro checkout. All three or none. Used only when managed.
    pub stripe: Option<StripeBilling>,
    pub link_check: linkdean_worker::LinkCheckOpts,
}

/// Stripe Billing for managed Pro. Price amounts live in the Stripe Dashboard, not here.
#[derive(Clone, Debug)]
pub struct StripeBilling {
    pub secret_key: String,
    pub webhook_secret: String,
    /// Stripe Price id for personal Pro (`price_…`).
    pub price_id_pro: String,
}

/// The operator's legal documents, from `TERMS_VERSION`, `TERMS_URL` and `PRIVACY_URL`. Without
/// a terms version, sign-up asks for no acceptance. The app links only the URLs that are set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Legal {
    pub terms_version: Option<String>,
    pub terms_url: Option<String>,
    pub privacy_url: Option<String>,
}

impl Legal {
    /// Reads the three settings through `var`. A URL must be http(s), since the app links to it
    /// and `/terms` redirects to it, and a terms version needs `TERMS_URL` to show the terms.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let get = |k: &str| {
            var(k)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let url = |k: &str| match get(k) {
            Some(u) if !(u.starts_with("https://") || u.starts_with("http://")) => {
                anyhow::bail!("{k} must start with https:// or http://")
            }
            u => Ok(u),
        };
        let legal = Self {
            terms_version: get("TERMS_VERSION"),
            terms_url: url("TERMS_URL")?,
            privacy_url: url("PRIVACY_URL")?,
        };
        if legal.terms_version.is_some() && legal.terms_url.is_none() {
            anyhow::bail!("TERMS_VERSION is set but TERMS_URL is missing");
        }
        Ok(legal)
    }
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
            archive_dir: std::env::var("ARCHIVE_DIR").unwrap_or_else(|_| "./data/archives".into()),
            archive_concurrency: env_usize("ARCHIVE_CONCURRENCY", 2).clamp(1, 8),
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
            oidc_name: std::env::var("OIDC_NAME")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            google_sso: optional_pair("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET")?,
            microsoft_sso: microsoft_sso()?,
            github_sso: optional_pair("GITHUB_CLIENT_ID", "GITHUB_CLIENT_SECRET")?,
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
            legal: Legal::from_vars(|k| std::env::var(k).ok())?,
            require_verified_email: env_bool("REQUIRE_VERIFIED_EMAIL", false),
            block_disposable_emails: env_bool("BLOCK_DISPOSABLE_EMAILS", false),
            dev_routes: env_bool("ALLOW_DEFAULT_SECRETS", false),
            experimental_agent_skills: env_bool("EXPERIMENTAL_AGENT_SKILLS", false),
            update_check: env_bool("UPDATE_CHECK", true),
            managed_service: env_bool("MANAGED_SERVICE", false),
            stripe: stripe_billing()?,
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

    /// Whether the generic OIDC provider (`OIDC_ISSUER` and its client) is set.
    pub fn generic_oidc_enabled(&self) -> bool {
        self.oidc_issuer.is_some()
            && self.oidc_client_id.is_some()
            && self.oidc_client_secret.is_some()
    }

    /// Whether any single sign-on provider is set.
    pub fn oidc_enabled(&self) -> bool {
        self.generic_oidc_enabled()
            || self.google_sso.is_some()
            || self.microsoft_sso.is_some()
            || self.github_sso.is_some()
    }

    /// The Terms of Service version sign-up must accept, if the server has terms.
    pub fn terms_version(&self) -> Option<&str> {
        self.legal.terms_version.as_deref()
    }

    pub fn turnstile_enabled(&self) -> bool {
        self.turnstile_site_key.is_some() && self.turnstile_secret_key.is_some()
    }

    /// Managed service with Stripe keys: Settings can open Checkout / Customer Portal.
    pub fn billing_enabled(&self) -> bool {
        self.managed_service && self.stripe.is_some()
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

/// Microsoft Entra ID app registration for a single tenant.
#[derive(Debug, Clone)]
pub struct MicrosoftSso {
    pub client_id: String,
    pub client_secret: String,
    /// Directory (tenant) ID. The issuer is per tenant, so `common` cannot be checked.
    pub tenant_id: String,
}

/// `MICROSOFT_CLIENT_ID`, `MICROSOFT_CLIENT_SECRET` and `MICROSOFT_TENANT_ID`: all three or none.
fn microsoft_sso() -> anyhow::Result<Option<MicrosoftSso>> {
    let Some((client_id, client_secret)) =
        optional_pair("MICROSOFT_CLIENT_ID", "MICROSOFT_CLIENT_SECRET")?
    else {
        if std::env::var("MICROSOFT_TENANT_ID").is_ok_and(|v| !v.trim().is_empty()) {
            anyhow::bail!("MICROSOFT_TENANT_ID is set but MICROSOFT_CLIENT_ID is missing");
        }
        return Ok(None);
    };
    let tenant_id = std::env::var("MICROSOFT_TENANT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("MICROSOFT_CLIENT_ID is set but MICROSOFT_TENANT_ID is missing")
        })?;
    if !is_tenant_id(&tenant_id) {
        anyhow::bail!(
            "MICROSOFT_TENANT_ID must be the directory (tenant) ID, a GUID; common, organizations \
             and consumers are not supported"
        );
    }
    Ok(Some(MicrosoftSso {
        client_id,
        client_secret,
        tenant_id,
    }))
}

/// A GUID such as `72f988bf-86f1-41af-91ab-2d7cd011db47`.
fn is_tenant_id(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok() && s.len() == 36
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

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `STRIPE_PRICE_ID_PRO`: all three or none.
fn stripe_billing() -> anyhow::Result<Option<StripeBilling>> {
    let secret_key = env_opt("STRIPE_SECRET_KEY");
    let webhook_secret = env_opt("STRIPE_WEBHOOK_SECRET");
    let price_id_pro = env_opt("STRIPE_PRICE_ID_PRO");
    match (secret_key, webhook_secret, price_id_pro) {
        (Some(secret_key), Some(webhook_secret), Some(price_id_pro)) => Ok(Some(StripeBilling {
            secret_key,
            webhook_secret,
            price_id_pro,
        })),
        (None, None, None) => Ok(None),
        _ => anyhow::bail!(
            "STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET and STRIPE_PRICE_ID_PRO must all be set together, or all unset"
        ),
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

    fn legal(vars: &[(&str, &str)]) -> anyhow::Result<Legal> {
        Legal::from_vars(|k| {
            vars.iter()
                .find(|(name, _)| *name == k)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn legal_settings_are_optional_and_checked() {
        assert_eq!(legal(&[]).unwrap(), Legal::default());
        let set = legal(&[
            ("TERMS_VERSION", "2026-11-15"),
            ("TERMS_URL", "https://example.com/terms"),
            ("PRIVACY_URL", " https://example.com/privacy "),
        ])
        .unwrap();
        assert_eq!(set.terms_version.as_deref(), Some("2026-11-15"));
        assert_eq!(
            set.privacy_url.as_deref(),
            Some("https://example.com/privacy")
        );
        // A privacy policy alone asks for no acceptance.
        assert!(legal(&[("PRIVACY_URL", "https://example.com/privacy")])
            .unwrap()
            .terms_version
            .is_none());
        assert!(legal(&[("TERMS_VERSION", "2026-11-15")]).is_err());
        assert!(legal(&[("TERMS_URL", "javascript:alert(1)")]).is_err());
        assert!(legal(&[("PRIVACY_URL", "example.com/privacy")]).is_err());
    }

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
