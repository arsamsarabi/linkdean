//! Email verification, email change and the admin verification controls.
//!
//! An account counts as verified once it proved it controls its address. Invites only reach
//! verified accounts directly; invites for an unverified account's address wait as address-only
//! invites and attach here, once the address is verified.

use chrono::Duration;
use linkdean_core::crypto::verify_password;
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::account_tokens::{
    create_account_token, take_account_token, void_all_account_tokens, TokenPurpose,
};
use super::pending_shares::link_pending_share_invites;
use super::users::find_user_by_id;

/// How long a verification or email change link works.
pub const EMAIL_VERIFY_LINK_HOURS: i64 = 24;

const INVALID_VERIFY: &str = "this verification link has expired or was already used";
const INVALID_CHANGE: &str = "this email change link has expired or was already used";
const TAKEN: &str = "email already registered";

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Two accounts switching to one address at once: the loser gets `Conflict`, not a 500.
fn email_write_err(e: sqlx::Error) -> Error {
    match e.as_database_error() {
        Some(d) if d.is_unique_violation() => Error::Conflict(TAKEN.into()),
        _ => db_err(e),
    }
}

fn normalize_email(email: &str) -> Result<String> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("valid email required".into()));
    }
    Ok(email)
}

async fn email_taken_by_other(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    email: &str,
    user_id: Uuid,
) -> Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1 AND id <> $2)")
        .bind(email)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db_err)
}

/// Mark the account verified and attach invites waiting for its address. Returns how many
/// invites attached. Keeps the first verification time when already verified.
pub async fn mark_email_verified(pool: &PgPool, user_id: Uuid) -> Result<u32> {
    let email: String = sqlx::query_scalar(
        r#"
        UPDATE users
        SET email_verified_at = coalesce(email_verified_at, now()), updated_at = now()
        WHERE id = $1
        RETURNING email
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?
    .ok_or(Error::NotFound)?;
    link_pending_share_invites(pool, &email, user_id).await
}

/// Verify an address with its link. Works without a session: the token proves control of the
/// inbox. Returns the user id. The link works once.
pub async fn verify_email_token(pool: &PgPool, token: &str) -> Result<Uuid> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    let Some((user_id, _)) = take_account_token(&mut tx, token, TokenPurpose::VerifyEmail).await?
    else {
        return Err(Error::BadRequest(INVALID_VERIFY.into()));
    };
    let email: String = sqlx::query_scalar(
        "UPDATE users SET email_verified_at = coalesce(email_verified_at, now()), \
         updated_at = now() WHERE id = $1 RETURNING email",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    // Outside the transaction: the verification stands even if attaching an invite fails.
    link_pending_share_invites(pool, &email, user_id).await?;
    Ok(user_id)
}

/// Start an email change: checks the current password, refuses SSO-only accounts and
/// addresses that belong to another account. Returns the token for the link sent to the new
/// address. A newer request voids the older one. Nothing changes until
/// [`confirm_email_change`].
pub async fn start_email_change(
    pool: &PgPool,
    user_id: Uuid,
    password: &str,
    new_email: &str,
) -> Result<String> {
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let hash = user.password_hash.clone().ok_or_else(|| {
        Error::BadRequest("your identity provider manages this account's email".into())
    })?;
    let new_email = normalize_email(new_email)?;
    if new_email == user.email {
        return Err(Error::BadRequest("that is already your email".into()));
    }
    let password = password.to_string();
    let ok = super::blocking::run_kdf(move || verify_password(&password, &hash)).await?;
    if !ok {
        return Err(Error::Unauthorized);
    }
    if super::users::find_user_by_email(pool, &new_email)
        .await?
        .is_some()
    {
        return Err(Error::Conflict(TAKEN.into()));
    }
    create_account_token(
        pool,
        user_id,
        TokenPurpose::ChangeEmail,
        Duration::hours(EMAIL_VERIFY_LINK_HOURS),
        None,
        Some(&new_email),
    )
    .await
}

/// Switch `user_id` to `new_email` inside `tx`, verified, and void every link sent to the
/// old address. Returns the old address.
async fn switch_email(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    new_email: &str,
) -> Result<String> {
    let old_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    if email_taken_by_other(tx, new_email, user_id).await? {
        return Err(Error::Conflict(TAKEN.into()));
    }
    sqlx::query(
        "UPDATE users SET email = $2, email_verified_at = now(), updated_at = now() \
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(new_email)
    .execute(&mut **tx)
    .await
    .map_err(email_write_err)?;
    void_all_account_tokens(tx, user_id).await?;
    Ok(old_email)
}

/// Finish an email change with its link: checks again that the address is free, switches to
/// it and marks it verified, then attaches invites waiting for it. Returns
/// `(user_id, old_email, new_email)`. Links sent to the old address stop working.
pub async fn confirm_email_change(pool: &PgPool, token: &str) -> Result<(Uuid, String, String)> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    let Some((user_id, Some(new_email))) =
        take_account_token(&mut tx, token, TokenPurpose::ChangeEmail).await?
    else {
        return Err(Error::BadRequest(INVALID_CHANGE.into()));
    };
    let old_email = switch_email(&mut tx, user_id, &new_email).await?;
    tx.commit().await.map_err(email_write_err)?;
    link_pending_share_invites(pool, &new_email, user_id).await?;
    Ok((user_id, old_email, new_email))
}

/// Admin: mark an account verified (attaching waiting invites) or remove its verification.
/// Removing it keeps shares already accepted; only new invites wait.
pub async fn set_email_verified(pool: &PgPool, user_id: Uuid, verified: bool) -> Result<()> {
    if verified {
        mark_email_verified(pool, user_id).await?;
        return Ok(());
    }
    let found =
        sqlx::query("UPDATE users SET email_verified_at = NULL, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(db_err)?
            .rows_affected();
    if found == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// Admin: change an account's email at once, without a link. The admin vouches for the
/// address, so it counts as verified, and invites waiting for it attach.
pub async fn admin_change_email(pool: &PgPool, user_id: Uuid, new_email: &str) -> Result<()> {
    let new_email = normalize_email(new_email)?;
    let mut tx = pool.begin().await.map_err(db_err)?;
    switch_email(&mut tx, user_id, &new_email).await?;
    tx.commit().await.map_err(email_write_err)?;
    link_pending_share_invites(pool, &new_email, user_id).await?;
    Ok(())
}
