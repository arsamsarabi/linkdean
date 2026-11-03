//! Password reset links. The token is shown once; only its hash is stored.

use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_password, hash_token, MIN_SECRET_LEN};
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// What a successful reset ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetOutcome {
    pub user_id: Uuid,
    pub signed_out_sessions: u64,
    pub revoked_api_keys: u64,
}

const INVALID: &str = "this reset link has expired or was already used";

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// A new link for `user_id`, valid for `ttl`. Any earlier unused link stops working.
pub async fn create_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    ttl: Duration,
    created_by: Option<Uuid>,
) -> Result<String> {
    let token = hex::encode(crypto::random_bytes(32));
    let mut tx = pool.begin().await.map_err(db_err)?;
    // Lock the user so two requests at once cannot both leave a live link behind.
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    void_reset_tokens(&mut tx, user_id).await?;
    sqlx::query(
        "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at, created_by) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(hash_token(&token))
    .bind(Utc::now() + ttl)
    .bind(created_by)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(token)
}

/// Mark every unused link of `user_id` used, so none of them works any more.
pub(crate) async fn void_reset_tokens(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
) -> Result<()> {
    sqlx::query(
        "UPDATE password_reset_tokens SET used_at = now() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Set a new password with a reset link: ends every session of the user and, with
/// `revoke_api_keys`, revokes their API keys. The link works once.
pub async fn consume_reset_token(
    pool: &PgPool,
    token: &str,
    new_password: &str,
    revoke_api_keys: bool,
) -> Result<ResetOutcome> {
    if new_password.len() < MIN_SECRET_LEN {
        return Err(Error::BadRequest(format!(
            "password min {MIN_SECRET_LEN} chars"
        )));
    }
    let token_hash = hash_token(token);
    let new_password = new_password.to_string();
    let new_hash = super::blocking::run_kdf(move || hash_password(&new_password)).await?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    // FOR UPDATE: a second use of the same link waits here, then finds it used.
    let user_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT t.user_id FROM password_reset_tokens t
        JOIN users u ON u.id = t.user_id
        WHERE t.token_hash = $1 AND t.used_at IS NULL AND t.expires_at > now()
          AND u.password_hash IS NOT NULL
        FOR UPDATE OF t
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?;
    let Some(user_id) = user_id else {
        return Err(Error::BadRequest(INVALID.into()));
    };
    void_reset_tokens(&mut tx, user_id).await?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .bind(new_hash)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let signed_out_sessions = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?
        .rows_affected();
    let revoked_api_keys = if revoke_api_keys {
        sqlx::query(
            "UPDATE api_keys SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?
        .rows_affected()
    } else {
        0
    };
    tx.commit().await.map_err(db_err)?;
    Ok(ResetOutcome {
        user_id,
        signed_out_sessions,
        revoked_api_keys,
    })
}

/// Delete links used or expired more than a day ago.
pub async fn purge_old_reset_tokens(pool: &PgPool) -> Result<u64> {
    Ok(sqlx::query(
        "DELETE FROM password_reset_tokens \
         WHERE coalesce(used_at, expires_at) < now() - interval '1 day'",
    )
    .execute(pool)
    .await
    .map_err(db_err)?
    .rows_affected())
}
