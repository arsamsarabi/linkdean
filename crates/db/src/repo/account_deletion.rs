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

/// Days between asking to delete the account and the purge.
pub const ACCOUNT_DELETION_GRACE_DAYS: i64 = 7;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Schedules the purge `ACCOUNT_DELETION_GRACE_DAYS` from now and returns when it happens.
///
/// `confirm_email` must match the account email (case-insensitive). Accounts with a local
/// password must also give it; single sign-on only accounts have none, so the email alone is
/// the gate (Known issue: no fresh SSO re-authentication yet). Refuses the last active admin.
/// Already scheduled accounts keep their date.
pub async fn schedule_account_deletion(
    pool: &PgPool,
    user_id: Uuid,
    confirm_email: &str,
    password: Option<String>,
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

/// Deletes a user whose grace period has ended and returns the ids of the bookmarks they owned,
/// so the caller can drop their search entries and preview files. Returns nothing, and deletes
/// nothing, when the account was cancelled or is gone since [`list_users_due_for_deletion`].
pub async fn purge_account(pool: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    let due: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM users
        WHERE id = $1 AND deletion_scheduled_at IS NOT NULL AND deletion_scheduled_at <= now()
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_err)?;
    if due.is_none() {
        return Ok(Vec::new());
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
    Ok(bookmark_ids)
}
