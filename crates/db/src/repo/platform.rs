use chrono::{DateTime, Utc};
use linkdean_core::{Error, PlatformSettings, Result};
use sqlx::AssertSqlSafe;
use sqlx::PgPool;

const PLATFORM_COLS: &str = r#"
    registration_open, api_keys_enabled, require_verified_email,
    max_bookmarks, max_stored_image_bytes, max_enrich_actions_per_day,
    max_import_bytes, max_upload_image_bytes, abuse_contact_email,
    screenshot_retry_cooldown_hours, screenshot_auto_retry_days, screenshot_retry_last_run_at,
    updated_at
"#;

pub async fn get_platform_settings(pool: &PgPool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        SELECT {PLATFORM_COLS}
        FROM platform_settings
        WHERE id = TRUE
        "#
    )))
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

pub async fn set_registration_open(pool: &PgPool, open: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        UPDATE platform_settings
        SET registration_open = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    )))
    .bind(open)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

pub async fn set_api_keys_enabled(pool: &PgPool, enabled: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        UPDATE platform_settings
        SET api_keys_enabled = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    )))
    .bind(enabled)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Whether unverified accounts are held at the verify screen. The caller checks that mail is
/// on before turning it on.
pub async fn set_require_verified_email(pool: &PgPool, on: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        UPDATE platform_settings
        SET require_verified_email = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    )))
    .bind(on)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Patch optional quota columns. Each `Some(v)` writes `v` (`None` clears the limit).
pub async fn set_platform_quotas(
    pool: &PgPool,
    max_bookmarks: Option<Option<i64>>,
    max_stored_image_bytes: Option<Option<i64>>,
    max_enrich_actions_per_day: Option<Option<i64>>,
    max_import_bytes: Option<Option<i64>>,
    max_upload_image_bytes: Option<Option<i64>>,
) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        UPDATE platform_settings
        SET
            max_bookmarks = CASE WHEN $1::bool THEN $2 ELSE max_bookmarks END,
            max_stored_image_bytes = CASE WHEN $3::bool THEN $4 ELSE max_stored_image_bytes END,
            max_enrich_actions_per_day = CASE WHEN $5::bool THEN $6 ELSE max_enrich_actions_per_day END,
            max_import_bytes = CASE WHEN $7::bool THEN $8 ELSE max_import_bytes END,
            max_upload_image_bytes = CASE WHEN $9::bool THEN $10 ELSE max_upload_image_bytes END,
            updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    )))
    .bind(max_bookmarks.is_some())
    .bind(max_bookmarks.flatten())
    .bind(max_stored_image_bytes.is_some())
    .bind(max_stored_image_bytes.flatten())
    .bind(max_enrich_actions_per_day.is_some())
    .bind(max_enrich_actions_per_day.flatten())
    .bind(max_import_bytes.is_some())
    .bind(max_import_bytes.flatten())
    .bind(max_upload_image_bytes.is_some())
    .bind(max_upload_image_bytes.flatten())
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Set (`Some`) or clear (`None`) where abuse reports can be sent. The caller validates it.
pub async fn set_abuse_contact_email(
    pool: &PgPool,
    email: Option<&str>,
) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        UPDATE platform_settings
        SET abuse_contact_email = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    )))
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Change the screenshot retry cooldown (hours) or automatic retry window (days). `None`
/// leaves a value alone. Values out of range are refused by the table's checks.
pub async fn set_screenshot_retry_settings(
    pool: &PgPool,
    cooldown_hours: Option<i32>,
    auto_retry_days: Option<i32>,
) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(AssertSqlSafe(format!(
        r#"
        UPDATE platform_settings
        SET screenshot_retry_cooldown_hours = COALESCE($1, screenshot_retry_cooldown_hours),
            screenshot_auto_retry_days = COALESCE($2, screenshot_auto_retry_days),
            updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    )))
    .bind(cooldown_hours)
    .bind(auto_retry_days)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Start a manual screenshot retry if the cooldown has passed since the last one. Returns
/// the start time, or `None` while the cooldown runs. One statement, so two admins clicking
/// at once start one retry.
pub async fn claim_screenshot_retry(pool: &PgPool) -> Result<Option<DateTime<Utc>>> {
    sqlx::query_scalar(
        r#"
        UPDATE platform_settings
        SET screenshot_retry_last_run_at = now()
        WHERE id = TRUE
          AND (screenshot_retry_last_run_at IS NULL
               OR screenshot_retry_last_run_at
                  <= now() - make_interval(hours => screenshot_retry_cooldown_hours))
        RETURNING screenshot_retry_last_run_at
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}
