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
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    /// Empty = reflect `APP_URL` only. Use `*` for any-origin (dev/homelab convenience).
    pub cors_origins: Vec<String>,
    /// Behind a reverse proxy, read the client address from `X-Forwarded-For`.
    pub trust_forwarded_for: bool,
    pub link_check: linkdean_worker::LinkCheckOpts,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
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
            oidc_issuer: std::env::var("OIDC_ISSUER").ok().filter(|s| !s.is_empty()),
            oidc_client_id: std::env::var("OIDC_CLIENT_ID")
                .ok()
                .filter(|s| !s.is_empty()),
            oidc_client_secret: std::env::var("OIDC_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
            cors_origins,
            trust_forwarded_for: env_bool("TRUST_FORWARDED_FOR", false),
            link_check: linkdean_worker::LinkCheckOpts {
                enabled: env_bool("LINK_CHECK_ENABLED", true),
                tick_secs: env_u64("LINK_CHECK_TICK_SECS", 60),
                batch_size: env_i64("LINK_CHECK_BATCH_SIZE", 40),
                concurrency: env_usize("LINK_CHECK_CONCURRENCY", 2),
                per_host_delay_ms: env_u64("LINK_CHECK_PER_HOST_DELAY_MS", 1_000),
                recheck_days: env_i64("LINK_CHECK_RECHECK_DAYS", 7),
            },
        })
    }

    pub fn oidc_enabled(&self) -> bool {
        self.oidc_issuer.is_some()
            && self.oidc_client_id.is_some()
            && self.oidc_client_secret.is_some()
    }

    pub fn cookie_secure(&self) -> bool {
        self.app_url.starts_with("https://")
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("missing env {key}"))
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
