#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub meili_url: String,
    pub meili_key: String,
    pub app_url: String,
    pub session_secret: String,
    pub session_days: i64,
    pub avatar_dir: String,
    pub preview_dir: String,
    pub browser_ws_url: Option<String>,
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind: std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            database_url: required("DATABASE_URL")?,
            meili_url: std::env::var("MEILI_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:7700".into()),
            meili_key: std::env::var("MEILI_MASTER_KEY")
                .unwrap_or_else(|_| "dev_master_key_change_me".into()),
            app_url: std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:8080".into()),
            session_secret: required("SESSION_SECRET")?,
            session_days: std::env::var("SESSION_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            avatar_dir: std::env::var("AVATAR_DIR").unwrap_or_else(|_| "./data/avatars".into()),
            preview_dir: std::env::var("PREVIEW_DIR").unwrap_or_else(|_| "./data/previews".into()),
            browser_ws_url: std::env::var("BROWSER_WS_URL").ok().filter(|s| !s.is_empty()),
            oidc_issuer: std::env::var("OIDC_ISSUER").ok().filter(|s| !s.is_empty()),
            oidc_client_id: std::env::var("OIDC_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            oidc_client_secret: std::env::var("OIDC_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
        })
    }

    pub fn oidc_enabled(&self) -> bool {
        self.oidc_issuer.is_some()
            && self.oidc_client_id.is_some()
            && self.oidc_client_secret.is_some()
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("missing env {key}"))
}
