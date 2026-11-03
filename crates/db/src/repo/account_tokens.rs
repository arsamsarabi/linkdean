//! One-time account links: password reset, email verification and email change. The token is
//! shown once; only its hash is stored. Each user has at most one live token per purpose.

use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_password, hash_token, MIN_SECRET_LEN};
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// What a token is for. A token only works for the purpose it was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    PasswordReset,
    VerifyEmail,
    ChangeEmail,
}

impl TokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PasswordReset => "password_reset",
            Self::VerifyEmail => "verify_email",
            Self::ChangeEmail => "change_email",
        }
    }
}

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

/// A new `purpose` link for `user_id`, valid for `ttl`. Any earlier unused link for the same
/// purpose stops working; links for other purposes are left alone. `new_email` is the address
/// an email change switches to, and is required for [`TokenPurpose::ChangeEmail`] only.
pub async fn create_account_token(
    pool: &PgPool,
    user_id: Uuid,
    purpose: TokenPurpose,
    ttl: Duration,
    created_by: Option<Uuid>,
    new_email: Option<&str>,
) -> Result<String> {
    if (purpose == TokenPurpose::ChangeEmail) != new_email.is_some() {
        return Err(Error::msg("new_email is set for change_email tokens only"));
    }
    let token = hex::encode(crypto::random_bytes(32));
    let mut tx = pool.begin().await.map_err(db_err)?;
    // Lock the user so two requests at once cannot both leave a live link behind.
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    void_account_tokens(&mut tx, user_id, purpose).await?;
    sqlx::query(
        "INSERT INTO account_tokens \
         (user_id, token_hash, expires_at, created_by, purpose, new_email) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(hash_token(&token))
    .bind(Utc::now() + ttl)
    .bind(created_by)
    .bind(purpose.as_str())
    .bind(new_email)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(token)
}

/// A new password reset link for `user_id`, valid for `ttl`. Any earlier unused reset link
/// stops working.
pub async fn create_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    ttl: Duration,
    created_by: Option<Uuid>,
) -> Result<String> {
    create_account_token(
        pool,
        user_id,
        TokenPurpose::PasswordReset,
        ttl,
        created_by,
        None,
    )
    .await
}

/// Mark every unused `purpose` link of `user_id` used, so none of them works any more.
pub(crate) async fn void_account_tokens(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    purpose: TokenPurpose,
) -> Result<()> {
    sqlx::query(
        "UPDATE account_tokens SET used_at = now() \
         WHERE user_id = $1 AND purpose = $2 AND used_at IS NULL",
    )
    .bind(user_id)
    .bind(purpose.as_str())
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Mark every unused link of `user_id` used, whatever its purpose. For when the address the
/// links were sent to stops being the account's.
pub(crate) async fn void_all_account_tokens(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
) -> Result<()> {
    sqlx::query("UPDATE account_tokens SET used_at = now() WHERE user_id = $1 AND used_at IS NULL")
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Use up a live `purpose` token inside `tx`: returns its user and `new_email`, and marks
/// every live token of that user and purpose used. `None` when the token is unknown, expired,
/// used, or for another purpose. The row lock makes a second use wait, then find it used.
/// A reset token only counts while the account still has a local password.
pub(crate) async fn take_account_token(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    token: &str,
    purpose: TokenPurpose,
) -> Result<Option<(Uuid, Option<String>)>> {
    let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
        r#"
        SELECT t.user_id, t.new_email FROM account_tokens t
        JOIN users u ON u.id = t.user_id
        WHERE t.token_hash = $1 AND t.purpose = $2
          AND t.used_at IS NULL AND t.expires_at > now()
          AND (t.purpose <> 'password_reset' OR u.password_hash IS NOT NULL)
        FOR UPDATE OF t
        "#,
    )
    .bind(hash_token(token))
    .bind(purpose.as_str())
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_err)?;
    if let Some((user_id, _)) = &row {
        void_account_tokens(tx, *user_id, purpose).await?;
    }
    Ok(row)
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
    let new_password = new_password.to_string();
    let new_hash = super::blocking::run_kdf(move || hash_password(&new_password)).await?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    let Some((user_id, _)) =
        take_account_token(&mut tx, token, TokenPurpose::PasswordReset).await?
    else {
        return Err(Error::BadRequest(INVALID.into()));
    };
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

/// Delete links of every purpose used or expired more than a day ago.
pub async fn purge_old_account_tokens(pool: &PgPool) -> Result<u64> {
    Ok(sqlx::query(
        "DELETE FROM account_tokens \
         WHERE coalesce(used_at, expires_at) < now() - interval '1 day'",
    )
    .execute(pool)
    .await
    .map_err(db_err)?
    .rows_affected())
}
