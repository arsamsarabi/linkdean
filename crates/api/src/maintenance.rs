//! Housekeeping that runs at startup and then on a fixed interval.

use crate::state::AppState;
use std::path::Path;
use std::time::Duration;

/// How often trash retention, the session purge, the unused tag reap, the old email purge, the old
/// reset link purge, the old pending single sign-on link purge and the vault key sweep run.
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

/// Hard-deletes accounts whose deletion grace period has ended, then drops what lives outside
/// Postgres: search entries, preview files and the avatar. One failing account does not stop
/// the rest; a failed purge is retried on the next tick because the schedule stays set.
// Known issue: the last-admin check runs when deletion is scheduled, not here, so admins who
// each schedule deletion while another admin exists can leave the instance with none. Fix: re-check
// under the `user_bans` lock in `purge_account` and skip the purge.
async fn purge_deleted_accounts(state: &AppState) {
    let due = match linkdean_db::list_users_due_for_deletion(&state.pool).await {
        Ok(due) => due,
        Err(e) => {
            tracing::warn!(error = %e, "listing accounts due for deletion failed");
            return;
        }
    };
    let preview_dir = Path::new(&state.cfg.preview_dir);
    for user_id in due {
        let bookmark_ids = match linkdean_db::purge_account(&state.pool, user_id).await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!(%user_id, error = %e, "account purge failed");
                continue;
            }
        };
        // Keys of the deleted user's sessions are unreachable now, but free the memory.
        state.clear_vault_keys(user_id).await;
        for id in &bookmark_ids {
            if let Err(e) = state.search.delete_bookmark(*id).await {
                tracing::warn!(bookmark_id = %id, error = %e, "meili delete on account purge failed");
            }
            linkdean_worker::purge_preview_assets(preview_dir, *id).await;
        }
        let avatar = state.avatar_path(user_id);
        if let Err(e) = tokio::fs::remove_file(&avatar).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(%user_id, error = %e, "avatar delete on account purge failed");
            }
        }
        tracing::info!(%user_id, bookmarks = bookmark_ids.len(), "purged deleted account");
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

    purge_deleted_accounts(state).await;

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

    match linkdean_db::purge_old_emails(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(emails = n, "purged old outbox email"),
        Err(e) => tracing::warn!(error = %e, "outbox purge failed"),
    }

    match linkdean_db::purge_old_account_tokens(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(tokens = n, "purged old account links"),
        Err(e) => tracing::warn!(error = %e, "account link purge failed"),
    }

    match linkdean_db::purge_old_pending_oidc_links(&state.pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(links = n, "purged old pending single sign-on links"),
        Err(e) => tracing::warn!(error = %e, "pending single sign-on link purge failed"),
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
