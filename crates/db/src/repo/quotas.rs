//! Per-user quota gauges and period counters. Limits come from `resolve_limits` (platform today).

use chrono::{NaiveDate, Utc};
use linkdean_core::{Error, ResolvedLimits, Result};
use sqlx::PgPool;
use uuid::Uuid;

pub const METER_BOOKMARKS: &str = "bookmarks";
pub const METER_STORED_IMAGES: &str = "stored_images";
pub const METER_ENRICH: &str = "enrich";
pub const METER_IMPORT: &str = "import";
pub const METER_UPLOAD: &str = "upload";

/// Today UTC as the enrich period start (membership can change this later).
pub fn enrich_period_start_utc() -> NaiveDate {
    Utc::now().date_naive()
}

pub fn resolve_limits(settings: &linkdean_core::PlatformSettings) -> ResolvedLimits {
    ResolvedLimits::from(settings)
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
}
