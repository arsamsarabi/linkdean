use crate::config::Config;
use linkdean_core::VaultKeyring;
use linkdean_db::{self as db};
use linkdean_search::SearchClient;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub search: SearchClient,
    pub cfg: Config,
    /// user_id → unlocked vaults (session memory; lost on restart)
    pub vault_keys: Arc<RwLock<HashMap<Uuid, VaultKeyring>>>,
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
        Ok(Self {
            pool,
            search,
            cfg,
            vault_keys: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn vault_keyring(&self, user_id: Uuid) -> VaultKeyring {
        self.vault_keys
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn set_vault_key(&self, user_id: Uuid, vault_id: Uuid, key: [u8; 32]) {
        self.vault_keys
            .write()
            .await
            .entry(user_id)
            .or_default()
            .insert(vault_id, key);
    }

    pub async fn clear_vault_keys(&self, user_id: Uuid) {
        self.vault_keys.write().await.remove(&user_id);
    }
}
