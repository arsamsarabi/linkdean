//! Single sign-on identities on accounts, and pending links: a single sign-on sign-in whose
//! address belongs to a password account waits here until a password sign-in to that account
//! attaches it. The browser holds the token; only its hash is stored. A link works once.

use chrono::{DateTime, Duration, Utc};
use linkdean_core::crypto::{self, hash_token, verify_password};
use linkdean_core::{Error, Result};
use sqlx::{Acquire, PgPool};
use uuid::Uuid;

use super::pending_shares::link_pending_share_invites;
use super::users::find_user_by_id;

/// How long a pending single sign-on link works.
pub const PENDING_OIDC_LINK_MINUTES: i64 = 10;

const IDENTITY_TAKEN: &str = "This sign-in is already connected to another account.";
const ACCOUNT_HAS_IDENTITY: &str = "This account is already connected to a different sign-in.";

/// A single sign-on identity waiting to be attached to an account.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingOidcLink {
    pub id: Uuid,
    pub issuer: String,
    pub subject: String,
    /// The address the provider reported, as it reported it.
    pub email: String,
    /// Whether the provider said it verified `email`.
    pub email_verified: bool,
    pub expires_at: DateTime<Utc>,
}

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Two accounts taking one identity at once: the loser gets `Conflict`, not a 500.
fn identity_write_err(e: sqlx::Error) -> Error {
    match e.as_database_error() {
        Some(d) if d.is_unique_violation() => Error::Conflict(IDENTITY_TAKEN.into()),
        _ => db_err(e),
    }
}

/// Store a pending link for the provider identity `issuer` + `subject` and return its token,
/// valid for [`PENDING_OIDC_LINK_MINUTES`].
pub async fn create_pending_oidc_link(
    pool: &PgPool,
    issuer: &str,
    subject: &str,
    email: &str,
    email_verified: bool,
) -> Result<String> {
    let token = hex::encode(crypto::random_bytes(32));
    sqlx::query(
        "INSERT INTO pending_oidc_links \
         (token_hash, issuer, subject, email, email_verified, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(hash_token(&token))
    .bind(issuer)
    .bind(subject)
    .bind(email)
    .bind(email_verified)
    .bind(Utc::now() + Duration::minutes(PENDING_OIDC_LINK_MINUTES))
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(token)
}

/// The live pending link for `token`, without using it. `None` when unknown, expired or used.
pub async fn peek_pending_oidc_link(pool: &PgPool, token: &str) -> Result<Option<PendingOidcLink>> {
    sqlx::query_as::<_, PendingOidcLink>(
        "SELECT id, issuer, subject, email, email_verified, expires_at FROM pending_oidc_links \
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await
    .map_err(db_err)
}

/// Attach `issuer` + `subject` to `user_id` inside `tx`. Refuses an identity that belongs to
/// another account and an account that already has a different identity. Attaching the
/// identity the account already has is a no-op.
async fn link_identity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    issuer: &str,
    subject: &str,
) -> Result<()> {
    let current: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT oidc_issuer, oidc_subject FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db_err)?
            .ok_or(Error::NotFound)?;
    match current {
        (Some(i), Some(s)) if i == issuer && s == subject => return Ok(()),
        (_, Some(_)) => return Err(Error::Conflict(ACCOUNT_HAS_IDENTITY.into())),
        _ => {}
    }
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM users \
         WHERE oidc_issuer = $1 AND oidc_subject = $2 AND id <> $3)",
    )
    .bind(issuer)
    .bind(subject)
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    if taken {
        return Err(Error::Conflict(IDENTITY_TAKEN.into()));
    }
    sqlx::query(
        "UPDATE users SET oidc_issuer = $2, oidc_subject = $3, updated_at = now() WHERE id = $1",
    )
    .bind(user_id)
    .bind(issuer)
    .bind(subject)
    .execute(&mut **tx)
    .await
    .map_err(identity_write_err)?;
    Ok(())
}

/// Attach the provider identity `issuer` + `subject` to `user_id`. `Conflict` when another
/// account has it or this account already has a different one. Leaves the address and its
/// verification alone.
pub async fn link_oidc_identity(
    pool: &PgPool,
    user_id: Uuid,
    issuer: &str,
    subject: &str,
) -> Result<()> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    link_identity(&mut tx, user_id, issuer, subject).await?;
    tx.commit().await.map_err(identity_write_err)
}

/// Use up the pending link for `token` after `user_id` signed in with a password. Attaches the
/// identity only when the account's address is the provider's (case-insensitive); otherwise the
/// link is spent and nothing is linked (`Ok(None)`). `Ok(None)` too when the link is unknown,
/// expired or used. When attaching is refused (`Conflict`), the link is spent as well. With a
/// provider-verified address the account counts as verified and waiting invites attach.
pub async fn consume_pending_oidc_link(
    pool: &PgPool,
    token: &str,
    user_id: Uuid,
) -> Result<Option<PendingOidcLink>> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    // The row lock makes a second use wait, then find it used.
    let Some(link) = sqlx::query_as::<_, PendingOidcLink>(
        "SELECT id, issuer, subject, email, email_verified, expires_at FROM pending_oidc_links \
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now() FOR UPDATE",
    )
    .bind(hash_token(token))
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?
    else {
        return Ok(None);
    };
    sqlx::query("UPDATE pending_oidc_links SET used_at = now() WHERE id = $1")
        .bind(link.id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
    let Some(email) = email.filter(|e| e.trim().eq_ignore_ascii_case(link.email.trim())) else {
        tx.commit().await.map_err(db_err)?;
        return Ok(None);
    };

    // A savepoint, so a refused link still leaves the pending link spent.
    let mut attach = (&mut tx).begin().await.map_err(db_err)?;
    if let Err(e) = link_identity(&mut attach, user_id, &link.issuer, &link.subject).await {
        attach.rollback().await.map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        return Err(e);
    }
    attach.commit().await.map_err(identity_write_err)?;
    if link.email_verified {
        sqlx::query(
            "UPDATE users SET email_verified_at = coalesce(email_verified_at, now()), \
             updated_at = now() WHERE id = $1",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(identity_write_err)?;
    if link.email_verified {
        // Outside the transaction: the link stands even if attaching an invite fails.
        if let Err(e) = link_pending_share_invites(pool, &email, user_id).await {
            tracing::warn!(%user_id, error = %e, "attaching invites after sso link failed");
        }
    }
    Ok(Some(link))
}

/// Disconnect single sign-on from `user_id` after checking the current password. Refuses
/// accounts without a local password (`BadRequest`), so nobody loses their only way in, a
/// wrong password (`Unauthorized`), and accounts with nothing connected (`BadRequest`).
pub async fn unlink_oidc_identity(pool: &PgPool, user_id: Uuid, password: &str) -> Result<()> {
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let hash = user
        .password_hash
        .clone()
        .ok_or_else(|| Error::BadRequest("account has no local password".into()))?;
    let password = password.to_string();
    let ok = super::blocking::run_kdf(move || verify_password(&password, &hash)).await?;
    if !ok {
        return Err(Error::Unauthorized);
    }
    let cleared = sqlx::query(
        "UPDATE users SET oidc_issuer = NULL, oidc_subject = NULL, updated_at = now() \
         WHERE id = $1 AND oidc_subject IS NOT NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(db_err)?
    .rows_affected();
    if cleared == 0 {
        return Err(Error::BadRequest("single sign-on is not connected".into()));
    }
    Ok(())
}

/// Delete pending links used or expired more than a day ago.
pub async fn purge_old_pending_oidc_links(pool: &PgPool) -> Result<u64> {
    Ok(sqlx::query(
        "DELETE FROM pending_oidc_links \
         WHERE coalesce(used_at, expires_at) < now() - interval '1 day'",
    )
    .execute(pool)
    .await
    .map_err(db_err)?
    .rows_affected())
}
