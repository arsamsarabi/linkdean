//! Housekeeping that runs at startup and then on a fixed interval.

use crate::state::AppState;
use std::path::Path;
use std::time::Duration;

/// How often trash retention, the session purge, the unused tag reap and the vault key sweep run.
const INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Run housekeeping now, then every [`INTERVAL`], for the life of the process.
pub async fn run(state: AppState) {
    let mut tick = tokio::time::interval(INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        run_once(&state).await;
    }
}

async fn run_once(state: &AppState) {
    match linkdean_db::purge_expired_trash(&state.pool).await {
        Ok(purged) => {
            let preview_dir = Path::new(&state.cfg.preview_dir);
            for id in &purged.bookmark_ids {
                if let Err(e) = state.search.delete_bookmark(*id).await {
                    tracing::warn!(bookmark_id = %id, error = %e, "meili delete on retention purge failed");
                }
                linkdean_worker::purge_preview_assets(preview_dir, *id).await;
            }
            let bookmarks = purged.bookmark_ids.len() as u64;
            if bookmarks + purged.categories + purged.dashboards > 0 {
                tracing::info!(
                    bookmarks,
                    categories = purged.categories,
                    dashboards = purged.dashboards,
                    "purged expired trash"
                );
            }
        }
        Err(e) => tracing::warn!(error = %e, "trash retention purge failed"),
    }

    match linkdean_db::purge_expired_sessions(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(sessions = n, "purged expired sessions"),
        Err(e) => tracing::warn!(error = %e, "session purge failed"),
    }

    match linkdean_db::reap_unused_tags(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(tags = n, "removed unused tags"),
        Err(e) => tracing::warn!(error = %e, "unused tag reap failed"),
    }

    // Vault keys of sessions that ended or sat idle past the owner's auto-lock time.
    let hashes = state.vault_keys.session_hashes();
    match linkdean_db::live_session_hashes(&state.pool, &hashes).await {
        Ok(live) => {
            let dropped = state.vault_keys.sweep(&live);
            if dropped > 0 {
                tracing::info!(
                    sessions = dropped,
                    "locked vaults of ended or idle sessions"
                );
            }
        }
        Err(e) => tracing::warn!(error = %e, "vault key sweep failed"),
    }
}
