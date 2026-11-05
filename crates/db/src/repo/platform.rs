use linkdean_core::{Error, PlatformSettings, Result};
use sqlx::PgPool;

const PLATFORM_COLS: &str = r#"
    registration_open, api_keys_enabled, require_verified_email,
    max_bookmarks, max_stored_image_bytes, max_enrich_actions_per_day,
    max_import_bytes, max_upload_image_bytes, updated_at
"#;

pub async fn get_platform_settings(pool: &PgPool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(&format!(
        r#"
        SELECT {PLATFORM_COLS}
        FROM platform_settings
        WHERE id = TRUE
        "#
    ))
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

pub async fn set_registration_open(pool: &PgPool, open: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(&format!(
        r#"
        UPDATE platform_settings
        SET registration_open = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    ))
    .bind(open)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

pub async fn set_api_keys_enabled(pool: &PgPool, enabled: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(&format!(
        r#"
        UPDATE platform_settings
        SET api_keys_enabled = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    ))
    .bind(enabled)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Whether unverified accounts are held at the verify screen. The caller checks that mail is
/// on before turning it on.
pub async fn set_require_verified_email(pool: &PgPool, on: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(&format!(
        r#"
        UPDATE platform_settings
        SET require_verified_email = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING {PLATFORM_COLS}
        "#
    ))
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
    sqlx::query_as::<_, PlatformSettings>(&format!(
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
    ))
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
