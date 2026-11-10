//! Single-use codes that connect the browser extension to an account (see migration 050).

use linkdean_core::crypto::{constant_time_eq, hash_token, random_bytes};
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// How long the extension has to trade a code for a key. The trade follows the approval at once.
pub const EXTENSION_CODE_SECONDS: i64 = 120;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// What a code turns into once its PKCE verifier checks out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionGrant {
    pub user_id: Uuid,
    pub key_name: String,
}

/// Stores a new code for `user_id` and returns it. Expired codes of every account go first.
pub async fn create_extension_code(
    pool: &PgPool,
    user_id: Uuid,
    code_challenge: &str,
    extension_origin: &str,
    key_name: &str,
) -> Result<String> {
    sqlx::query("DELETE FROM extension_connect_codes WHERE expires_at <= now()")
        .execute(pool)
        .await
        .map_err(db_err)?;
    let code = hex::encode(random_bytes(32));
    sqlx::query(
        r#"
        INSERT INTO extension_connect_codes
            (code_hash, user_id, code_challenge, extension_origin, key_name, expires_at)
        VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))
        "#,
    )
    .bind(hash_token(&code))
    .bind(user_id)
    .bind(code_challenge)
    .bind(extension_origin)
    .bind(key_name)
    .bind(EXTENSION_CODE_SECONDS as f64)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(code)
}

/// Spends `code`. The code is gone after the first attempt, right or wrong, so a guessed
/// verifier gets one try. `None` for an unknown, expired or spent code, a different extension
/// address, or a verifier that does not match the challenge.
pub async fn take_extension_code(
    pool: &PgPool,
    code: &str,
    code_verifier: &str,
    extension_origin: &str,
) -> Result<Option<ExtensionGrant>> {
    let row: Option<(Uuid, String, String, String)> = sqlx::query_as(
        r#"
        DELETE FROM extension_connect_codes
        WHERE code_hash = $1 AND expires_at > now()
        RETURNING user_id, code_challenge, extension_origin, key_name
        "#,
    )
    .bind(hash_token(code.trim()))
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;
    let Some((user_id, challenge, stored_origin, key_name)) = row else {
        return Ok(None);
    };
    let computed = linkdean_core::extension_connect::pkce_challenge(code_verifier);
    if stored_origin != extension_origin
        || !constant_time_eq(computed.as_bytes(), challenge.as_bytes())
    {
        return Ok(None);
    }
    Ok(Some(ExtensionGrant { user_id, key_name }))
}
