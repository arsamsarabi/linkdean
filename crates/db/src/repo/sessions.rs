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

pub async fn delete_session(pool: &PgPool, token: &str) -> Result<()> {
    let token_hash = hash_token(token);
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}
