use crate::config::Config;
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
    /// user_id → vault data key (session memory; lost on restart)
    pub vault_keys: Arc<RwLock<HashMap<Uuid, [u8; 32]>>>,
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
        std::path::Path::new(&self.cfg.preview_dir).join(format!("{bookmark_id}.jpg"))
    }

    pub fn enrich_opts(&self) -> linkdean_worker::EnrichOpts {
        linkdean_worker::EnrichOpts {
            preview_dir: std::path::PathBuf::from(&self.cfg.preview_dir),
            browser_ws_url: self.cfg.browser_ws_url.clone(),
        }
    }

    pub async fn vault_key(&self, user_id: Uuid) -> Option<[u8; 32]> {
        self.vault_keys.read().await.get(&user_id).copied()
    }

    pub async fn set_vault_key(&self, user_id: Uuid, key: [u8; 32]) {
        self.vault_keys.write().await.insert(user_id, key);
    }

    pub async fn clear_vault_key(&self, user_id: Uuid) {
        self.vault_keys.write().await.remove(&user_id);
    }
}
