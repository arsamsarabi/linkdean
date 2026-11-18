//! Meili upserts that attach archive `page_text` (never loaded by ViewBatch).

use crate::state::AppState;
use linkdean_core::BookmarkView;
use linkdean_db as db;
use uuid::Uuid;

/// Upsert one bookmark after loading HTML archive page text when indexed.
pub async fn upsert(state: &AppState, view: &BookmarkView) -> anyhow::Result<()> {
    let enriched = enrich_page_texts(state, std::slice::from_ref(view)).await?;
    write_meili(state, &enriched).await
}

/// Upsert many bookmarks after loading HTML archive page texts when indexed.
pub async fn upsert_many(state: &AppState, views: &[BookmarkView]) -> anyhow::Result<()> {
    let enriched = enrich_page_texts(state, views).await?;
    write_meili(state, &enriched).await
}

async fn enrich_page_texts(
    state: &AppState,
    views: &[BookmarkView],
) -> anyhow::Result<Vec<BookmarkView>> {
    let ids: Vec<_> = views
        .iter()
        .filter(|v| !v.is_vault && v.deleted_at.is_none())
        .map(|v| v.id)
        .collect();
    let mut texts = if ids.is_empty() {
        Default::default()
    } else {
        // Fail closed: never upsert with empty page_text after a load error (Meili wipe).
        db::html_page_texts(&state.pool, &ids)
            .await
            .map_err(|e| anyhow::anyhow!("html page texts load failed: {e}"))?
    };
    Ok(views
        .iter()
        .cloned()
        .map(|mut v| {
            if let Some(t) = texts.remove(&v.id) {
                v.page_text = t;
            }
            v
        })
        .collect())
}

/// Re-check live vault/trash flags immediately before Meili write (post-move TOCTOU).
async fn write_meili(state: &AppState, views: &[BookmarkView]) -> anyhow::Result<()> {
    let ids: Vec<Uuid> = views.iter().map(|v| v.id).collect();
    let gates = db::bookmark_index_gates(&state.pool, &ids)
        .await
        .map_err(|e| anyhow::anyhow!("bookmark index gates failed: {e}"))?;

    let mut to_upsert = Vec::with_capacity(views.len());
    for view in views {
        match gates.get(&view.id) {
            Some((false, false)) => to_upsert.push(view.clone()),
            Some((true, _)) | Some((_, true)) | None => {
                // Vault, trash, or gone: remove rather than re-insert page_text.
                state.search.delete_bookmark(view.id).await?;
            }
        }
    }
    if to_upsert.is_empty() {
        return Ok(());
    }
    if to_upsert.len() == 1 {
        state.search.upsert_bookmark(&to_upsert[0]).await
    } else {
        state.search.upsert_bookmarks(&to_upsert).await
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn rust_files_under(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files_under(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn upsert_call_sites_go_through_search_index() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files_under(&root, &mut files);

        let mut offenders = Vec::new();
        let mut admin_uses_helper = false;
        for path in &files {
            let rel = path.strip_prefix(&root).unwrap_or(path);
            let text = fs::read_to_string(path).unwrap_or_default();
            if rel == Path::new("search_index.rs") {
                continue;
            }
            if text.contains("upsert_bookmark") {
                offenders.push(rel.display().to_string());
            }
            if rel.starts_with("routes/admin") && text.contains("search_index::") {
                admin_uses_helper = true;
            }
        }
        assert!(
            offenders.is_empty(),
            "direct SearchClient upsert outside search_index.rs: {offenders:?}"
        );
        assert!(
            admin_uses_helper,
            "admin reindex must call search_index:: upsert helper"
        );
    }
}
