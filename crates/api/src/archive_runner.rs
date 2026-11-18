//! Works through the archive queue: one visit per bookmark makes every format asked for. Copies
//! go to `ARCHIVE_DIR/{bookmark_id}/`, vault copies encrypted with the vault's key. The queue
//! lives in Postgres, so a restart picks up where it stopped.

use crate::state::AppState;
use linkdean_core::{crypto, ArchiveFormat, VaultKeyring};
use linkdean_db::{self as db, StoredCopy};
use linkdean_worker::archive::{self, mhtml::Limits, CaptureError, Probe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

/// Largest single resource kept inside an HTML copy.
const MAX_RESOURCE: usize = 5 * 1024 * 1024;
const MIB: i64 = 1024 * 1024;
const SWEEP_EVERY: Duration = Duration::from_secs(3600);
/// Files younger than this may belong to a capture still being recorded.
const SWEEP_MIN_AGE: Duration = Duration::from_secs(3600);

pub async fn run(state: AppState) {
    match db::requeue_running_archives(&state.pool).await {
        Ok(0) => {}
        Ok(n) => info!(n, "requeued archives cut off by a restart"),
        Err(e) => warn!(error = %e, "archive requeue failed"),
    }
    tokio::spawn(sweep_loop(state.clone()));
    let slots = Arc::new(tokio::sync::Semaphore::new(state.cfg.archive_concurrency));
    loop {
        let Ok(permit) = slots.clone().acquire_owned().await else {
            return;
        };
        match db::claim_next_archive(&state.pool).await {
            Ok(Some((id, formats))) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(reason) = capture(&state, id, &formats).await {
                        if let Err(e) = db::fail_archive(&state.pool, id, &formats, &reason).await {
                            warn!(bookmark_id = %id, error = %e, "recording an archive failure failed");
                        }
                    }
                    drop(permit);
                });
            }
            Ok(None) => {
                drop(permit);
                let _ =
                    tokio::time::timeout(Duration::from_secs(30), state.archive_wake.notified())
                        .await;
            }
            Err(e) => {
                drop(permit);
                warn!(error = %e, "archive claim failed");
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }
    }
}

/// The address to capture and, for a vault bookmark, its vault and key.
struct Target {
    url: String,
    owner: Uuid,
    vault: Option<(Uuid, [u8; 32])>,
}

async fn target(state: &AppState, id: Uuid) -> Result<Target, String> {
    let b = db::archive_target(&state.pool, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("The bookmark was deleted.")?;
    if b.deleted_at.is_some() {
        return Err("The bookmark is in Trash.".into());
    }
    let (url, vault) = match (b.is_vault, b.vault_id) {
        (true, Some(vault_id)) => {
            let key = state
                .vault_keys
                .user_key(b.owner_user_id, vault_id)
                .ok_or("Unlock the vault, then archive again.")?;
            let (ct, nonce) = (
                b.vault_ciphertext.clone().unwrap_or_default(),
                b.vault_nonce.clone().unwrap_or_default(),
            );
            let payload = crypto::decrypt_payload(&key, vault_id, id, &ct, &nonce)
                .map_err(|_| "Unlock the vault, then archive again.")?;
            (payload.url, Some((vault_id, key)))
        }
        _ => (b.url, None),
    };
    match url::Url::parse(&url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => Ok(Target {
            url,
            owner: b.owner_user_id,
            vault,
        }),
        _ => Err("Only web addresses can be archived.".into()),
    }
}

/// Captures one bookmark. `Err` is the reason shown to the person; any earlier copy stays.
async fn capture(state: &AppState, id: Uuid, formats: &[ArchiveFormat]) -> Result<(), String> {
    let t = target(state, id).await?;
    let settings = db::get_platform_settings(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    let cap = settings.max_archive_item_bytes.max(MIB) as usize;
    let too_big = || {
        format!(
            "This page is too big to archive (over {} MB).",
            settings.max_archive_item_bytes / MIB
        )
    };
    let explain = |e: CaptureError| match e {
        CaptureError::TooBig => too_big(),
        CaptureError::Gone(code) => format!("The page is gone (HTTP {code})."),
        CaptureError::Challenge => "The site showed a bot check instead of the page.".into(),
        CaptureError::Failed(e) => format!("The page could not be archived: {e}"),
    };

    let mut copies: Vec<(ArchiveFormat, Vec<u8>, String)> = Vec::new();
    let mut final_url = t.url.clone();
    match archive::probe(&t.url).await {
        Probe::Gone(code) => return Err(explain(CaptureError::Gone(code))),
        Probe::File { .. } => {
            let (bytes, ct) = archive::fetch_file(&t.url, cap).await.map_err(explain)?;
            copies.push((ArchiveFormat::File, bytes, ct));
        }
        Probe::Page => {
            let browser = state.cfg.browser_ws_url.as_deref().ok_or(
                "Archiving pages needs the browser service. Ask the server admin to set BROWSER_WS_URL.",
            )?;
            let limits = Limits {
                max_resource: MAX_RESOURCE.min(cap),
                max_total: cap,
            };
            let want = |f| formats.contains(&f);
            let got = archive::capture_page(
                browser,
                &t.url,
                want(ArchiveFormat::Html),
                want(ArchiveFormat::Pdf),
                &limits,
            )
            .await
            .map_err(explain)?;
            final_url = got.final_url;
            if let Some(html) = got.html {
                copies.push((ArchiveFormat::Html, html, "text/html; charset=utf-8".into()));
            }
            if let Some(pdf) = got.pdf {
                copies.push((ArchiveFormat::Pdf, pdf, "application/pdf".into()));
            }
        }
    }

    if let Some(limit) = settings.max_archive_bytes {
        let used = db::archive_bytes_for_owner(&state.pool, t.owner)
            .await
            .map_err(|e| e.to_string())?;
        let replaced: i64 = db::list_archives(&state.pool, id)
            .await
            .map_err(|e| e.to_string())?
            .iter()
            .map(|a| a.bytes)
            .sum();
        let adding: i64 = copies.iter().map(|(_, b, _)| b.len() as i64).sum();
        if used - replaced + adding > limit {
            return Err(db::ARCHIVE_STORAGE_FULL.into());
        }
    }

    let dir = state.archive_dir(id);
    let mut stored = Vec::new();
    for (format, bytes, content_type) in copies {
        let (body, key_vault_id, nonce) = match t.vault {
            Some((vault_id, key)) => {
                let (ct, nonce) = crypto::encrypt_archive(&key, vault_id, id, format, &bytes)
                    .map_err(|e| e.to_string())?;
                (ct, Some(vault_id), Some(nonce))
            }
            None => (bytes.clone(), None, None),
        };
        let file_name = match write_file(&dir, format, &body).await {
            Ok(name) => name,
            Err(e) => {
                remove_files(&dir, stored.iter().map(|c: &StoredCopy| &c.file_name)).await;
                return Err(format!("The copy could not be saved: {e}"));
            }
        };
        stored.push(StoredCopy {
            format,
            file_name,
            bytes: bytes.len() as i64,
            content_type,
            final_url: t.vault.is_none().then(|| final_url.clone()),
            key_vault_id,
            nonce,
            // Task 5 wires extract_page_text for non-vault HTML.
            page_text: None,
        });
    }
    let unused = match db::finish_archive(&state.pool, id, &stored).await {
        Ok(unused) => unused,
        Err(e) => {
            remove_files(&dir, stored.iter().map(|c| &c.file_name)).await;
            return Err(e.to_string());
        }
    };
    remove_files(&dir, unused.iter()).await;
    // Formats asked for but not made: a file stands in for both page formats.
    let missing: Vec<ArchiveFormat> = formats
        .iter()
        .copied()
        .filter(|f| {
            !stored
                .iter()
                .any(|c| c.format == *f || c.format == ArchiveFormat::File)
        })
        .collect();
    if !missing.is_empty() {
        let _ =
            db::fail_archive(&state.pool, id, &missing, "The page could not be archived.").await;
    }
    Ok(())
}

/// Writes `bytes` under a fresh name, fully on disk before the name appears.
async fn write_file(dir: &Path, format: ArchiveFormat, bytes: &[u8]) -> std::io::Result<String> {
    tokio::fs::create_dir_all(dir).await?;
    let name = format!("{}-{}", format.as_str(), Uuid::new_v4().simple());
    let tmp = dir.join(format!("{name}.tmp"));
    let result = async {
        let mut f = tokio::fs::File::create(&tmp).await?;
        f.write_all(bytes).await?;
        f.sync_all().await?;
        tokio::fs::rename(&tmp, dir.join(&name)).await
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    result.map(|_| name)
}

pub async fn remove_files<'a>(dir: &Path, names: impl Iterator<Item = &'a String>) {
    for name in names {
        if let Err(e) = tokio::fs::remove_file(dir.join(name)).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!(?dir, %name, error = %e, "archive file delete failed");
            }
        }
    }
}

/// Re-encrypt copies after a move into, out of or between vaults, with keys the mover's session
/// holds. A copy whose key is missing stays as it was, still readable with its old vault's key.
pub async fn rekey_after_vault_move(state: AppState, keyring: VaultKeyring, owner_id: Uuid) {
    let rows = match db::archives_needing_rekey(&state.pool, owner_id).await {
        Ok(rows) => rows,
        Err(e) => {
            warn!(error = %e, "archive re-key lookup failed");
            return;
        }
    };
    for (row, target) in rows {
        let (Ok(format), Some(old)) = (row.format(), row.file_name.clone()) else {
            continue;
        };
        let dir = state.archive_dir(row.bookmark_id);
        let result = async {
            let bytes = tokio::fs::read(dir.join(&old))
                .await
                .map_err(|e| e.to_string())?;
            let plain = match (row.key_vault_id, &row.nonce) {
                (Some(v), Some(nonce)) => {
                    let key = keyring.get(v).ok_or("the old vault is locked")?;
                    crypto::decrypt_archive(key, v, row.bookmark_id, format, &bytes, nonce)
                        .map_err(|e| e.to_string())?
                }
                _ => bytes,
            };
            let (body, nonce) = match target {
                Some(v) => {
                    let key = keyring.get(v).ok_or("the new vault is locked")?;
                    let (ct, nonce) =
                        crypto::encrypt_archive(key, v, row.bookmark_id, format, &plain)
                            .map_err(|e| e.to_string())?;
                    (ct, Some(nonce))
                }
                None => (plain, None),
            };
            let new = write_file(&dir, format, &body)
                .await
                .map_err(|e| e.to_string())?;
            let swapped = db::set_archive_key(
                &state.pool,
                row.bookmark_id,
                format,
                &old,
                &new,
                target,
                nonce,
            )
            .await
            .map_err(|e| e.to_string())?;
            let unused = if swapped { old.clone() } else { new };
            remove_files(&dir, std::iter::once(&unused)).await;
            Ok::<(), String>(())
        }
        .await;
        if let Err(e) = result {
            warn!(bookmark_id = %row.bookmark_id, error = %e, "archive re-key failed");
        }
    }
}

/// Removes files no record points at: copies of purged bookmarks (Trash, folders, accounts) and
/// leftovers of interrupted captures or re-keys.
async fn sweep_loop(state: AppState) {
    loop {
        tokio::time::sleep(SWEEP_EVERY).await;
        match sweep(&state).await {
            Ok(0) => {}
            Ok(n) => info!(n, "removed unused archive files"),
            Err(e) => warn!(error = %e, "archive sweep failed"),
        }
    }
}

async fn sweep(state: &AppState) -> anyhow::Result<usize> {
    let root = PathBuf::from(&state.cfg.archive_dir);
    let known = db::archive_file_names(&state.pool).await?;
    let mut removed = 0;
    let mut dirs = tokio::fs::read_dir(&root).await?;
    while let Some(dir) = dirs.next_entry().await? {
        let Some(id) = dir
            .file_name()
            .to_str()
            .and_then(|s| Uuid::parse_str(s).ok())
        else {
            continue;
        };
        let keep = known.get(&id);
        let mut files = tokio::fs::read_dir(dir.path()).await?;
        let mut left = 0;
        while let Some(file) = files.next_entry().await? {
            let name = file.file_name().to_string_lossy().to_string();
            let fresh = file
                .metadata()
                .await?
                .modified()?
                .elapsed()
                .is_ok_and(|age| age < SWEEP_MIN_AGE);
            if fresh || keep.is_some_and(|k| k.contains(&name)) {
                left += 1;
                continue;
            }
            tokio::fs::remove_file(file.path()).await?;
            removed += 1;
        }
        if left == 0 {
            let _ = tokio::fs::remove_dir(dir.path()).await;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_written_file_appears_whole_under_a_fresh_name() {
        let dir = std::env::temp_dir().join(format!("ld-archive-{}", Uuid::new_v4()));
        let a = write_file(&dir, ArchiveFormat::Html, b"one").await.unwrap();
        let b = write_file(&dir, ArchiveFormat::Html, b"two").await.unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("html-"));
        assert_eq!(tokio::fs::read(dir.join(&a)).await.unwrap(), b"one");
        let mut names = Vec::new();
        let mut entries = tokio::fs::read_dir(&dir).await.unwrap();
        while let Some(e) = entries.next_entry().await.unwrap() {
            names.push(e.file_name().to_string_lossy().to_string());
        }
        assert!(names.iter().all(|n| !n.ends_with(".tmp")), "{names:?}");
        remove_files(&dir, [a, b].iter()).await;
        tokio::fs::remove_dir(&dir).await.unwrap();
    }
}
