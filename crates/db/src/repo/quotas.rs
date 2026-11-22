//! Per-user quota gauges and period counters. Limits come from platform ± managed plan.

use chrono::{NaiveDate, Utc};
use linkdean_core::{merge_limits, AccountPlan, Error, ResolvedLimits, Result};
use sqlx::PgPool;
use uuid::Uuid;

pub const METER_BOOKMARKS: &str = "bookmarks";
pub const METER_STORED_IMAGES: &str = "stored_images";
pub const METER_ENRICH: &str = "enrich";
pub const METER_IMPORT: &str = "import";
pub const METER_UPLOAD: &str = "upload";
pub const METER_ARCHIVE: &str = "archive";
pub const METER_VAULTS: &str = "vaults";

/// Today UTC as the enrich period start (membership can change this later).
pub fn enrich_period_start_utc() -> NaiveDate {
    Utc::now().date_naive()
}

/// Platform row alone, or merged with a managed plan when `plan` is set.
pub fn resolve_limits(
    settings: &linkdean_core::PlatformSettings,
    plan: Option<AccountPlan>,
) -> ResolvedLimits {
    merge_limits(ResolvedLimits::from(settings), plan)
}

pub async fn get_user_plan(pool: &PgPool, user_id: Uuid) -> Result<Option<AccountPlan>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT plan FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .flatten();
    Ok(raw.as_deref().and_then(AccountPlan::parse))
}

pub async fn resolve_limits_for(pool: &PgPool, user_id: Uuid) -> Result<ResolvedLimits> {
    let settings = super::platform::get_platform_settings(pool).await?;
    let plan = get_user_plan(pool, user_id).await?;
    Ok(resolve_limits(&settings, plan))
}

pub async fn list_owned_bookmark_ids(pool: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT id FROM bookmarks
        WHERE owner_user_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn count_owned_bookmarks(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM bookmarks
        WHERE owner_user_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n)
}

pub async fn count_vaults(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let n: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint FROM vaults WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n)
}

/// Refuse when adding `additional` bookmarks would exceed the gauge.
pub async fn refuse_bookmark_quota(
    pool: &PgPool,
    user_id: Uuid,
    limits: &ResolvedLimits,
    additional: i64,
) -> Result<()> {
    let Some(limit) = limits.max_bookmarks else {
        return Ok(());
    };
    let used = count_owned_bookmarks(pool, user_id).await?;
    if used.saturating_add(additional) > limit {
        return Err(Error::QuotaExceeded {
            meter: METER_BOOKMARKS,
            limit,
            used,
        });
    }
    Ok(())
}

/// Refuse when creating another vault would exceed the plan/platform vault cap.
pub async fn refuse_vault_quota(
    pool: &PgPool,
    user_id: Uuid,
    limits: &ResolvedLimits,
) -> Result<()> {
    let Some(limit) = limits.max_vaults else {
        return Ok(());
    };
    let used = count_vaults(pool, user_id).await?;
    if used.saturating_add(1) > limit {
        return Err(Error::QuotaExceeded {
            meter: METER_VAULTS,
            limit,
            used,
        });
    }
    Ok(())
}

pub async fn enrich_usage_today(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let period = enrich_period_start_utc();
    let n: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT count FROM user_quota_usage
        WHERE user_id = $1 AND meter = $2 AND period_start = $3
        "#,
    )
    .bind(user_id)
    .bind(METER_ENRICH)
    .bind(period)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.unwrap_or(0))
}

/// Consume one enrich action for today. No-op when the limit is unset.
pub async fn consume_enrich_action(
    pool: &PgPool,
    user_id: Uuid,
    limits: &ResolvedLimits,
) -> Result<()> {
    let Some(limit) = limits.max_enrich_actions_per_day else {
        return Ok(());
    };
    let period = enrich_period_start_utc();
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO user_quota_usage (user_id, meter, period_start, count)
        VALUES ($1, $2, $3, 1)
        ON CONFLICT (user_id, meter, period_start)
        DO UPDATE SET count = user_quota_usage.count + 1
        WHERE user_quota_usage.count < $4
        RETURNING count
        "#,
    )
    .bind(user_id)
    .bind(METER_ENRICH)
    .bind(period)
    .bind(limit)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if row.is_none() {
        let used = enrich_usage_today(pool, user_id).await?;
        return Err(Error::QuotaExceeded {
            meter: METER_ENRICH,
            limit,
            used,
        });
    }
    Ok(())
}

pub fn refuse_if_over_bytes(
    meter: &'static str,
    limit: Option<i64>,
    used: i64,
    additional: i64,
) -> Result<()> {
    let Some(limit) = limit else {
        return Ok(());
    };
    if used.saturating_add(additional) > limit {
        return Err(Error::QuotaExceeded { meter, limit, used });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_if_over_bytes_respects_null_limit() {
        assert!(refuse_if_over_bytes(METER_UPLOAD, None, 0, 9_000_000).is_ok());
        assert!(refuse_if_over_bytes(METER_UPLOAD, Some(100), 90, 20).is_err());
        assert!(refuse_if_over_bytes(METER_UPLOAD, Some(100), 80, 20).is_ok());
    }

    #[test]
    fn resolve_limits_merges_free_plan() {
        let settings = linkdean_core::PlatformSettings {
            registration_open: true,
            api_keys_enabled: true,
            require_verified_email: false,
            organizations_create_open: true,
            max_bookmarks: None,
            max_stored_image_bytes: None,
            max_enrich_actions_per_day: None,
            max_import_bytes: None,
            max_upload_image_bytes: None,
            abuse_contact_email: None,
            screenshot_retry_cooldown_hours: 24,
            screenshot_auto_retry_days: 7,
            screenshot_retry_last_run_at: None,
            archives_enabled: true,
            max_archive_bytes: None,
            max_archive_item_bytes: 25 * 1024 * 1024,
            updated_at: chrono::Utc::now(),
        };
        let free = resolve_limits(&settings, Some(AccountPlan::Free));
        assert_eq!(free.max_bookmarks, Some(5_000));
        assert_eq!(free.max_vaults, Some(1));
        let self_host = resolve_limits(&settings, None);
        assert!(self_host.max_bookmarks.is_none());
        assert!(self_host.max_vaults.is_none());
    }
}
