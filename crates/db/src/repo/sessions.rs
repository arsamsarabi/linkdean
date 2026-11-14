use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_token};
use linkdean_core::{Error, Result, User};
use sqlx::PgPool;
use uuid::Uuid;

use super::users::find_user_by_id;

pub async fn create_session(pool: &PgPool, user_id: Uuid, days: i64) -> Result<String> {
    let token = hex::encode(crypto::random_bytes(32));
    let token_hash = hash_token(&token);
    let expires = Utc::now() + Duration::days(days);
    sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(token)
}

pub async fn user_from_token(pool: &PgPool, token: &str) -> Result<Option<User>> {
    // API keys use `ldk_` prefix; sessions are plain hex.
    if token.starts_with("ldk_") {
        return super::api_keys::user_from_api_key(pool, token).await;
    }
    let token_hash = hash_token(token);
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT user_id FROM sessions
        WHERE token_hash = $1 AND expires_at > now()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    match row {
        Some((uid,)) => find_user_by_id(pool, uid).await,
        None => Ok(None),
    }
}

/// Minutes a fresh sign-in in a session counts for steps that need one.
pub const REAUTH_WINDOW_MINUTES: i64 = 10;

/// Records that the person behind the session with `token_hash` just signed in again with their
/// provider. Only that session gains the step-up; it lasts [`REAUTH_WINDOW_MINUTES`].
pub async fn mark_session_reauthenticated(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
) -> Result<()> {
    let done = sqlx::query(
        r#"
        UPDATE sessions SET reauthenticated_at = now()
        WHERE token_hash = $1 AND user_id = $2 AND expires_at > now()
        "#,
    )
    .bind(token_hash)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if done.rows_affected() == 0 {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

/// Whether the session with `token_hash` signed in again within [`REAUTH_WINDOW_MINUTES`].
pub async fn session_recently_reauthenticated(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
) -> Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM sessions
            WHERE token_hash = $1 AND user_id = $2 AND expires_at > now()
              AND reauthenticated_at > now() - make_interval(mins => $3::int)
        )
        "#,
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(REAUTH_WINDOW_MINUTES as i32)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn delete_session(pool: &PgPool, token: &str) -> Result<()> {
    let token_hash = hash_token(token);
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Which of `token_hashes` belong to sessions that have not ended.
pub async fn live_session_hashes(
    pool: &PgPool,
    token_hashes: &[String],
) -> Result<std::collections::HashSet<String>> {
    if token_hashes.is_empty() {
        return Ok(Default::default());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT token_hash FROM sessions WHERE token_hash = ANY($1) AND expires_at > now()",
    )
    .bind(token_hashes)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|(h,)| h).collect())
}

/// Remove sessions past their expiry. Returns how many were removed.
pub async fn purge_expired_sessions(pool: &PgPool) -> Result<u64> {
    let res = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(res.rows_affected())
}
