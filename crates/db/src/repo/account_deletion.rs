//! Self-serve account deletion: schedule, cancel, and the purge maintenance runs once the grace
//! period passes. Every table that references `users` cascades, so deleting the user row wipes
//! the rest (bookmarks, vaults, shares, sessions, API keys).

use super::moderation::active_admins_except;
use super::users::find_user_by_id;
use chrono::{DateTime, Utc};
use linkdean_core::crypto::verify_password;
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// The refusal a single sign-on only account gets until it signs in again. The web app matches
/// on it to offer the provider sign-in.
pub const REAUTH_REQUIRED: &str = "confirm it is you: sign in again with single sign-on";

/// Days between asking to delete the account and the purge.
pub const ACCOUNT_DELETION_GRACE_DAYS: i64 = 7;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Schedules the purge `ACCOUNT_DELETION_GRACE_DAYS` from now and returns when it happens.
///
/// `confirm_email` must match the account email (case-insensitive). Accounts with a local
/// password must also give it. Accounts without one must have signed in again with their
/// provider in this session (`session_token_hash`) within the last few minutes; see
/// [`super::sessions::mark_session_reauthenticated`]. Refuses the last active admin. Already
/// scheduled accounts keep their date.
pub async fn schedule_account_deletion(
    pool: &PgPool,
    user_id: Uuid,
    confirm_email: &str,
    password: Option<String>,
    session_token_hash: Option<&str>,
) -> Result<DateTime<Utc>> {
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    if !user.email.eq_ignore_ascii_case(confirm_email.trim()) {
        return Err(Error::BadRequest(
            "email does not match this account".into(),
        ));
    }
    if let Some(hash) = user.password_hash {
        let password = password
            .filter(|p| !p.is_empty())
            .ok_or_else(|| Error::BadRequest("password required".into()))?;
        let ok = super::blocking::run_kdf(move || verify_password(&password, &hash)).await?;
        if !ok {
            return Err(Error::Unauthorized);
        }
    } else {
        let fresh = match session_token_hash {
            Some(hash) => {
                super::sessions::session_recently_reauthenticated(pool, user_id, hash).await?
            }
            None => false,
        };
        if !fresh {
            return Err(Error::BadRequest(REAUTH_REQUIRED.into()));
        }
    }

    let mut tx = pool.begin().await.map_err(db_err)?;
    // Same lock as banning, so an admin cannot delete themselves while another is banned.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('user_bans'))")
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let (is_admin, scheduled): (bool, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT is_admin, deletion_scheduled_at FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?
    .ok_or(Error::NotFound)?;
    if let Some(at) = scheduled {
        return Ok(at);
    }
    if is_admin && active_admins_except(&mut tx, user_id).await? == 0 {
        return Err(Error::BadRequest("cannot delete the last admin".into()));
    }
    let at: DateTime<Utc> = sqlx::query_scalar(
        r#"
        UPDATE users
        SET deletion_scheduled_at = now() + make_interval(days => $2::int), updated_at = now()
        WHERE id = $1
        RETURNING deletion_scheduled_at
        "#,
    )
    .bind(user_id)
    .bind(ACCOUNT_DELETION_GRACE_DAYS as i32)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(at)
}

/// Clears a scheduled deletion. Errors when none is scheduled.
pub async fn cancel_account_deletion(pool: &PgPool, user_id: Uuid) -> Result<()> {
    let done = sqlx::query(
        r#"
        UPDATE users SET deletion_scheduled_at = NULL, updated_at = now()
        WHERE id = $1 AND deletion_scheduled_at IS NOT NULL
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    if done.rows_affected() == 0 {
        return Err(Error::BadRequest(
            "account is not scheduled for deletion".into(),
        ));
    }
    Ok(())
}

/// Users whose grace period has ended.
pub async fn list_users_due_for_deletion(pool: &PgPool) -> Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT id FROM users
        WHERE deletion_scheduled_at IS NOT NULL AND deletion_scheduled_at <= now()
        ORDER BY deletion_scheduled_at
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

/// What [`purge_account`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum PurgeOutcome {
    /// The account is gone; the ids of the bookmarks it owned, so the caller can drop their
    /// search entries and preview files.
    Purged(Vec<Uuid>),
    /// The account was cancelled or is gone since [`list_users_due_for_deletion`].
    NotDue,
    /// The account is the last active admin. It stays, still scheduled, and the purge tries
    /// again on the next run, so it goes once another admin exists.
    LastAdmin,
}

/// Deletes a user whose grace period has ended. Re-checks the last-admin rule under the same
/// lock as banning, because the admins who were there at scheduling time may be gone.
pub async fn purge_account(pool: &PgPool, user_id: Uuid) -> Result<PurgeOutcome> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('user_bans'))")
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let due: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT is_admin FROM users
        WHERE id = $1 AND deletion_scheduled_at IS NOT NULL AND deletion_scheduled_at <= now()
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?;
    let Some(is_admin) = due else {
        return Ok(PurgeOutcome::NotDue);
    };
    if is_admin && active_admins_except(&mut tx, user_id).await? == 0 {
        return Ok(PurgeOutcome::LastAdmin);
    }
    let bookmark_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM bookmarks WHERE owner_user_id = $1")
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(PurgeOutcome::Purged(bookmark_ids))
}
