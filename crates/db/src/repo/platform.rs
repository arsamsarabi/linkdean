use linkdean_core::{Error, PlatformSettings, Result};
use sqlx::PgPool;

pub async fn get_platform_settings(pool: &PgPool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(
        r#"
        SELECT registration_open, api_keys_enabled, require_verified_email, updated_at
        FROM platform_settings
        WHERE id = TRUE
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

pub async fn set_registration_open(pool: &PgPool, open: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(
        r#"
        UPDATE platform_settings
        SET registration_open = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING registration_open, api_keys_enabled, require_verified_email, updated_at
        "#,
    )
    .bind(open)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

pub async fn set_api_keys_enabled(pool: &PgPool, enabled: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(
        r#"
        UPDATE platform_settings
        SET api_keys_enabled = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING registration_open, api_keys_enabled, require_verified_email, updated_at
        "#,
    )
    .bind(enabled)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}

/// Whether unverified accounts are held at the verify screen. The caller checks that mail is
/// on before turning it on.
pub async fn set_require_verified_email(pool: &PgPool, on: bool) -> Result<PlatformSettings> {
    sqlx::query_as::<_, PlatformSettings>(
        r#"
        UPDATE platform_settings
        SET require_verified_email = $1, updated_at = now()
        WHERE id = TRUE
        RETURNING registration_open, api_keys_enabled, require_verified_email, updated_at
        "#,
    )
    .bind(on)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::msg("platform_settings row missing"))
}
