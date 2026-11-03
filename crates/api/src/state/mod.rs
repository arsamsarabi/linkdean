use crate::config::Config;
use crate::rate_limit::{Limiter, WINDOW};
use crate::vault_keys::{idle_limit, VaultKeyStore, VaultSession};
use linkdean_core::VaultKeyring;
use linkdean_db::{self as db};
use linkdean_search::SearchClient;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub search: SearchClient,
    pub cfg: Config,
    /// Unlocked vaults per login session (memory only; lost on restart).
    pub vault_keys: Arc<VaultKeyStore>,
    pub limiter: Arc<Limiter>,
    pub mail: Option<Arc<crate::mail::Mail>>,
}

impl AppState {
    pub async fn new(cfg: Config) -> anyhow::Result<Self> {
        let pool = db::connect(&cfg.database_url).await?;
        db::migrate(&pool).await?;
        std::fs::create_dir_all(&cfg.avatar_dir)?;
        std::fs::create_dir_all(&cfg.preview_dir)?;
        let search = SearchClient::new(&cfg.meili_url, &cfg.meili_key)?;
        if let Err(e) = search.ensure_index().await {
            tracing::warn!("meilisearch index setup: {e}");
        }
        let mail = match &cfg.mail {
            Some(mail_cfg) => Some(Arc::new(crate::mail::Mail::new(mail_cfg, &cfg.app_url)?)),
            None => None,
        };
        match &mail {
            Some(m) => {
                tracing::info!(host = %m.cfg.host, port = m.cfg.port, tls = m.cfg.tls.as_str(), "outbound email on")
            }
            None => tracing::info!("outbound email off: SMTP_HOST and MAIL_FROM unset"),
        }
        Ok(Self {
            pool,
            search,
            cfg,
            vault_keys: Arc::new(VaultKeyStore::default()),
            limiter: Arc::new(Limiter::new(WINDOW)),
            mail,
        })
    }

    pub fn avatar_path(&self, user_id: Uuid) -> std::path::PathBuf {
        std::path::Path::new(&self.cfg.avatar_dir).join(format!("{user_id}.webp"))
    }

    pub fn preview_path(&self, bookmark_id: Uuid) -> std::path::PathBuf {
        linkdean_worker::preview_file(std::path::Path::new(&self.cfg.preview_dir), bookmark_id)
    }

    pub fn favicon_path(&self, bookmark_id: Uuid) -> std::path::PathBuf {
        linkdean_worker::favicon_file(std::path::Path::new(&self.cfg.preview_dir), bookmark_id)
    }

    pub fn favicon_ct_path(&self, bookmark_id: Uuid) -> std::path::PathBuf {
        linkdean_worker::favicon_ct_file(std::path::Path::new(&self.cfg.preview_dir), bookmark_id)
    }

    pub fn og_path(&self, bookmark_id: Uuid) -> std::path::PathBuf {
        linkdean_worker::og_file(std::path::Path::new(&self.cfg.preview_dir), bookmark_id)
    }

    pub fn og_ct_path(&self, bookmark_id: Uuid) -> std::path::PathBuf {
        linkdean_worker::og_ct_file(std::path::Path::new(&self.cfg.preview_dir), bookmark_id)
    }

    pub fn enrich_opts(&self) -> linkdean_worker::EnrichOpts {
        linkdean_worker::EnrichOpts {
            preview_dir: std::path::PathBuf::from(&self.cfg.preview_dir),
            browser_ws_url: self.cfg.browser_ws_url.clone(),
        }
    }

    /// The vaults this session has unlocked. Empty for API keys.
    pub async fn vault_keyring(&self, session: &VaultSession, user_id: Uuid) -> VaultKeyring {
        self.vault_keys.keyring(session, user_id)
    }

    /// Unlock a vault for this session, until the user's auto-lock time passes unused.
    pub async fn set_vault_key(
        &self,
        session: &VaultSession,
        user_id: Uuid,
        vault_id: Uuid,
        key: [u8; 32],
    ) -> linkdean_core::Result<()> {
        let settings = db::get_user_settings(&self.pool, user_id).await?;
        let idle = idle_limit(settings.vault_auto_lock_minutes);
        self.vault_keys
            .insert(session, user_id, vault_id, key, idle)
    }

    /// Lock one vault in every session of the user.
    pub async fn clear_vault_key(&self, user_id: Uuid, vault_id: Uuid) {
        self.vault_keys.remove_vault(user_id, vault_id);
    }

    /// Lock every vault in every session of the user.
    pub async fn clear_vault_keys(&self, user_id: Uuid) {
        self.vault_keys.clear_user(user_id);
    }
}
