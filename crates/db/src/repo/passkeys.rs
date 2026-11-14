//! Passkeys and the WebAuthn ceremonies that add and use them. The credential and challenge
//! state are stored as the JSON webauthn-rs writes; this crate never looks inside.

use chrono::{DateTime, Utc};
use linkdean_core::{Error, PasskeySummary, Result};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Passkeys one account may hold.
pub const MAX_PASSKEYS_PER_USER: i64 = 20;
/// Minutes a WebAuthn ceremony may take.
pub const WEBAUTHN_CHALLENGE_MINUTES: i64 = 5;
/// Longest passkey name.
pub const MAX_PASSKEY_NAME: usize = 64;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// What a ceremony is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebauthnPurpose {
    Register,
    Login,
}

impl WebauthnPurpose {
    fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Login => "login",
        }
    }
}

/// Stores a ceremony's state and returns its id, which the browser sends back.
pub async fn create_webauthn_challenge(
    pool: &PgPool,
    user_id: Option<Uuid>,
    purpose: WebauthnPurpose,
    state: &Value,
) -> Result<Uuid> {
    sqlx::query_scalar(
        r#"
        INSERT INTO webauthn_challenges (user_id, purpose, state, expires_at)
        VALUES ($1, $2, $3, now() + make_interval(mins => $4::int))
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(purpose.as_str())
    .bind(state)
    .bind(WEBAUTHN_CHALLENGE_MINUTES as i32)
    .fetch_one(pool)
    .await
    .map_err(db_err)
}

/// Takes a ceremony's state, once. `None` when it is unknown, used, expired, for another purpose
/// or, for adding a passkey, started by another account.
pub async fn take_webauthn_challenge(
    pool: &PgPool,
    id: Uuid,
    purpose: WebauthnPurpose,
    user_id: Option<Uuid>,
) -> Result<Option<Value>> {
    sqlx::query_scalar(
        r#"
        DELETE FROM webauthn_challenges
        WHERE id = $1 AND purpose = $2 AND expires_at > now()
          AND user_id IS NOT DISTINCT FROM $3
        RETURNING state
        "#,
    )
    .bind(id)
    .bind(purpose.as_str())
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)
}

/// Drops ceremonies nobody finished. Returns how many.
pub async fn purge_expired_webauthn_challenges(pool: &PgPool) -> Result<u64> {
    sqlx::query("DELETE FROM webauthn_challenges WHERE expires_at <= now()")
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(db_err)
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::BadRequest("give the passkey a name".into()));
    }
    if name.chars().count() > MAX_PASSKEY_NAME {
        return Err(Error::BadRequest(format!(
            "passkey names are at most {MAX_PASSKEY_NAME} characters"
        )));
    }
    Ok(name.to_string())
}

/// Saves a new passkey. Refuses a credential already registered (to any account) and more than
/// [`MAX_PASSKEYS_PER_USER`].
pub async fn insert_passkey(
    pool: &PgPool,
    user_id: Uuid,
    credential_id: &[u8],
    name: &str,
    passkey: &Value,
) -> Result<PasskeySummary> {
    let name = clean_name(name)?;
    let mut tx = pool.begin().await.map_err(db_err)?;
    // Serializes adds per account, so two at once cannot pass the count together.
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passkeys WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
    if count >= MAX_PASSKEYS_PER_USER {
        return Err(Error::BadRequest(format!(
            "an account can have at most {MAX_PASSKEYS_PER_USER} passkeys"
        )));
    }
    let row: (Uuid, DateTime<Utc>) = sqlx::query_as(
        r#"
        INSERT INTO passkeys (user_id, credential_id, name, passkey)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at
        "#,
    )
    .bind(user_id)
    .bind(credential_id)
    .bind(&name)
    .bind(passkey)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e.as_database_error() {
        Some(d) if d.is_unique_violation() => {
            Error::Conflict("this passkey is already registered".into())
        }
        _ => db_err(e),
    })?;
    tx.commit().await.map_err(db_err)?;
    Ok(PasskeySummary {
        id: row.0,
        name,
        created_at: row.1,
        last_used_at: None,
    })
}

/// The account's passkeys, newest first.
pub async fn list_passkeys(pool: &PgPool, user_id: Uuid) -> Result<Vec<PasskeySummary>> {
    sqlx::query_as::<_, (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>)>(
        r#"
        SELECT id, name, created_at, last_used_at FROM passkeys
        WHERE user_id = $1 ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(id, name, created_at, last_used_at)| PasskeySummary {
                id,
                name,
                created_at,
                last_used_at,
            })
            .collect()
    })
    .map_err(db_err)
}

/// The credential IDs the account already has, so the browser does not register one twice.
pub async fn passkey_credential_ids(pool: &PgPool, user_id: Uuid) -> Result<Vec<Vec<u8>>> {
    sqlx::query_scalar("SELECT credential_id FROM passkeys WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(db_err)
}

/// The stored credential for `credential_id`, when it belongs to `user_id`.
pub async fn find_passkey(
    pool: &PgPool,
    user_id: Uuid,
    credential_id: &[u8],
) -> Result<Option<(Uuid, Value)>> {
    sqlx::query_as("SELECT id, passkey FROM passkeys WHERE user_id = $1 AND credential_id = $2")
        .bind(user_id)
        .bind(credential_id)
        .fetch_optional(pool)
        .await
        .map_err(db_err)
}

/// Records a sign-in with the passkey, with its updated counter and backup state.
pub async fn record_passkey_use(pool: &PgPool, id: Uuid, passkey: &Value) -> Result<()> {
    sqlx::query("UPDATE passkeys SET passkey = $2, last_used_at = now() WHERE id = $1")
        .bind(id)
        .bind(passkey)
        .execute(pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn rename_passkey(pool: &PgPool, user_id: Uuid, id: Uuid, name: &str) -> Result<()> {
    let name = clean_name(name)?;
    let done = sqlx::query("UPDATE passkeys SET name = $3 WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .bind(name)
        .execute(pool)
        .await
        .map_err(db_err)?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn delete_passkey(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let done = sqlx::query("DELETE FROM passkeys WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    if done.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}
