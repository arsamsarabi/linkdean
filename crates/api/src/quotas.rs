//! Filesystem-aware quota helpers (stored image bytes) and effective upload/import ceilings.

use crate::state::AppState;
use linkdean_core::{Error, QuotaUsageView, ResolvedLimits, Result};
use linkdean_db::{self as db};
use std::path::Path;
use uuid::Uuid;

pub const HARD_IMPORT_BYTES: usize = 10 * 1024 * 1024;
pub const HARD_AVATAR_BODY: usize = 6 * 1024 * 1024;
pub const HARD_PREVIEW_BODY: usize = 9 * 1024 * 1024;

pub async fn limits_for(pool: &sqlx::PgPool) -> Result<ResolvedLimits> {
    let settings = db::get_platform_settings(pool).await?;
    Ok(db::resolve_limits(&settings))
}

/// Avatar + preview/favicon/og files for the user's owned bookmarks.
pub async fn stored_image_bytes(state: &AppState, user_id: Uuid) -> Result<i64> {
    let mut total: u64 = 0;
    let avatar = state.avatar_path(user_id);
    total += file_len(&avatar).await;
    let ids = db::list_owned_bookmark_ids(&state.pool, user_id).await?;
    for id in ids {
        total += file_len(&state.preview_path(id)).await;
        total += file_len(&state.favicon_path(id)).await;
        total += file_len(&state.og_path(id)).await;
    }
    Ok(total.min(i64::MAX as u64) as i64)
}

async fn file_len(path: &Path) -> u64 {
    match tokio::fs::metadata(path).await {
        Ok(m) => m.len(),
        Err(_) => 0,
    }
}

pub fn effective_import_max(limits: &ResolvedLimits) -> usize {
    match limits.max_import_bytes {
        Some(n) if n > 0 => (n as usize).min(HARD_IMPORT_BYTES),
        _ => HARD_IMPORT_BYTES,
    }
}

pub fn effective_upload_max(limits: &ResolvedLimits, hard: usize) -> usize {
    match limits.max_upload_image_bytes {
        Some(n) if n > 0 => (n as usize).min(hard),
        _ => hard,
    }
}

pub async fn refuse_upload(
    state: &AppState,
    user_id: Uuid,
    limits: &ResolvedLimits,
    body_len: usize,
    hard: usize,
) -> Result<()> {
    let max = effective_upload_max(limits, hard);
    if body_len > max {
        return Err(Error::QuotaExceeded {
            meter: db::METER_UPLOAD,
            limit: max as i64,
            used: body_len as i64,
        });
    }
    if limits.max_stored_image_bytes.is_some() {
        let used = stored_image_bytes(state, user_id).await?;
        db::refuse_if_over_bytes(
            db::METER_STORED_IMAGES,
            limits.max_stored_image_bytes,
            used,
            body_len as i64,
        )?;
    }
    Ok(())
}

pub async fn usage_view(state: &AppState, user_id: Uuid) -> Result<QuotaUsageView> {
    let limits = limits_for(&state.pool).await?;
    let bookmarks = db::count_owned_bookmarks(&state.pool, user_id).await?;
    let stored_image_bytes = stored_image_bytes(state, user_id).await?;
    let enrich_actions_today = db::enrich_usage_today(&state.pool, user_id).await?;
    Ok(QuotaUsageView {
        limits,
        bookmarks,
        stored_image_bytes,
        enrich_actions_today,
        enrich_period_start: db::enrich_period_start_utc().to_string(),
    })
}
