//! Meili upserts that attach archive `page_text` (never loaded by ViewBatch).

use crate::state::AppState;
use linkdean_core::BookmarkView;
use linkdean_db as db;

/// Upsert one bookmark after loading HTML archive page text when indexed.
pub async fn upsert(state: &AppState, view: &BookmarkView) -> anyhow::Result<()> {
    upsert_many(state, std::slice::from_ref(view)).await
}

/// Upsert many bookmarks after loading HTML archive page texts when indexed.
pub async fn upsert_many(state: &AppState, views: &[BookmarkView]) -> anyhow::Result<()> {
    let plan = db::plan_meili_index_writes(&state.pool, views)
        .await
        .map_err(|e| anyhow::anyhow!("meili index plan failed: {e}"))?;
    for id in plan.delete {
        // Warn-and-continue so one vault/trash id does not abort an import batch.
        if let Err(e) = state.search.delete_bookmark(id).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili delete during index plan failed");
        }
    }
    if plan.upsert.is_empty() {
        return Ok(());
    }
    if plan.upsert.len() == 1 {
        state.search.upsert_bookmark(&plan.upsert[0]).await
    } else {
        state.search.upsert_bookmarks(&plan.upsert).await
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

    /// Direct `SearchClient::upsert_bookmark(s)` only in `search_index` (after the shared
    /// plan) or in the worker (after `plan_meili_index_writes`).
    #[test]
    fn upsert_call_sites_go_through_shared_plan() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace");
        let mut offenders = Vec::new();
        let mut admin_uses_helper = false;
        let mut worker_uses_plan = false;

        for crate_src in ["crates/api/src", "crates/worker/src"] {
            let root = workspace.join(crate_src);
            let mut files = Vec::new();
            rust_files_under(&root, &mut files);
            for path in files {
                let rel = path
                    .strip_prefix(&workspace)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());
                let text = fs::read_to_string(&path).unwrap_or_default();
                if !text.contains("upsert_bookmark") {
                    continue;
                }
                if rel.ends_with("crates/api/src/search_index.rs") {
                    continue;
                }
                if rel.ends_with("crates/worker/src/lib.rs") {
                    worker_uses_plan = text.contains("plan_meili_index_writes");
                    if !worker_uses_plan {
                        offenders.push(rel);
                    }
                    continue;
                }
                offenders.push(rel);
            }
        }

        let admin = workspace.join("crates/api/src/routes/admin");
        let mut admin_files = Vec::new();
        rust_files_under(&admin, &mut admin_files);
        for path in admin_files {
            let text = fs::read_to_string(&path).unwrap_or_default();
            if text.contains("search_index::") {
                admin_uses_helper = true;
            }
        }

        assert!(
            offenders.is_empty(),
            "direct SearchClient upsert without shared plan: {offenders:?}"
        );
        assert!(
            admin_uses_helper,
            "admin reindex must call search_index:: upsert helper"
        );
        assert!(
            worker_uses_plan,
            "worker enrich must call db::plan_meili_index_writes before Meili upsert"
        );
    }
}
