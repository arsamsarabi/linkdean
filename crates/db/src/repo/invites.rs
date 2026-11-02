use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_token};
use linkdean_core::{
    CreateRegistrationInvite, CreateRegistrationInviteResponse, Error, RegistrationInvite,
    RegistrationInviteView, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

const TOKEN_PREFIX: &str = "ldi_";

fn to_view(row: RegistrationInvite) -> RegistrationInviteView {
    let status = if row.revoked_at.is_some() {
        "revoked"
    } else if row.used_at.is_some() {
        "used"
    } else if row.expires_at <= Utc::now() {
        "expired"
    } else {
        "pending"
    };
    RegistrationInviteView {
        id: row.id,
        code_prefix: row.code_prefix,
        email: row.email,
        created_by_user_id: row.created_by_user_id,
        expires_at: row.expires_at,
        used_at: row.used_at,
        used_by_user_id: row.used_by_user_id,
        revoked_at: row.revoked_at,
        created_at: row.created_at,
        status: status.into(),
    }
}

pub async fn list_registration_invites(pool: &PgPool) -> Result<Vec<RegistrationInviteView>> {
    let rows = sqlx::query_as::<_, RegistrationInvite>(
        r#"
        SELECT *
        FROM registration_invites
        ORDER BY created_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(to_view).collect())
}

pub async fn create_registration_invite(
    pool: &PgPool,
    created_by: Uuid,
    req: CreateRegistrationInvite,
) -> Result<CreateRegistrationInviteResponse> {
    let days = req.expires_in_days.unwrap_or(7).clamp(1, 90);
    let expires_at = Utc::now() + Duration::days(days as i64);
    let email = req
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());

    let secret = hex::encode(crypto::random_bytes(32));
    let code = format!("{TOKEN_PREFIX}{secret}");
    let code_hash = hash_token(&code);
    let code_prefix = format!("{TOKEN_PREFIX}{}", &secret[..8.min(secret.len())]);

    let row = sqlx::query_as::<_, RegistrationInvite>(
        r#"
        INSERT INTO registration_invites
            (code_hash, code_prefix, email, created_by_user_id, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&code_hash)
    .bind(&code_prefix)
    .bind(&email)
    .bind(created_by)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok(CreateRegistrationInviteResponse {
        invite: to_view(row),
        code,
    })
}

pub async fn revoke_registration_invite(pool: &PgPool, id: Uuid) -> Result<()> {
    let res = sqlx::query(
        r#"
        UPDATE registration_invites
        SET revoked_at = now()
        WHERE id = $1 AND revoked_at IS NULL AND used_at IS NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// Validate a pending invite for registration. Returns invite id on success.
// Known issue: Concurrent registrations can all pass the redeemable check before one consumes the
// invite, and the user row survives a failed consume, so one invite can create several accounts.
// Fix: claim the invite and insert the user in one transaction.
pub async fn consume_registration_invite(
    pool: &PgPool,
    code: &str,
    email: &str,
    user_id: Uuid,
) -> Result<Uuid> {
    let code = code.trim();
    if code.is_empty() {
        return Err(Error::BadRequest("invite code required".into()));
    }
    let code_hash = hash_token(code);
    let row = sqlx::query_as::<_, RegistrationInvite>(
        r#"
        SELECT * FROM registration_invites
        WHERE code_hash = $1
        "#,
    )
    .bind(&code_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::BadRequest("invalid invite code".into()))?;

    if row.revoked_at.is_some() {
        return Err(Error::BadRequest("invite revoked".into()));
    }
    if row.used_at.is_some() {
        return Err(Error::BadRequest("invite already used".into()));
    }
    if row.expires_at <= Utc::now() {
        return Err(Error::BadRequest("invite expired".into()));
    }
    if let Some(bound) = row.email.as_deref() {
        if bound != email.trim().to_ascii_lowercase() {
            return Err(Error::BadRequest("invite email mismatch".into()));
        }
    }

    let updated = sqlx::query_as::<_, RegistrationInvite>(
        r#"
        UPDATE registration_invites
        SET used_at = now(), used_by_user_id = $2
        WHERE id = $1 AND used_at IS NULL AND revoked_at IS NULL
        RETURNING *
        "#,
    )
    .bind(row.id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::BadRequest("invite already used".into()))?;

    Ok(updated.id)
}

/// Validate invite for register without consuming (fast-fail before user create).
pub async fn assert_invite_redeemable(pool: &PgPool, code: &str, email: &str) -> Result<()> {
    let code = code.trim();
    if code.is_empty() {
        return Err(Error::BadRequest("invite code required".into()));
    }
    let code_hash = hash_token(code);
    let row = sqlx::query_as::<_, RegistrationInvite>(
        r#"
        SELECT * FROM registration_invites
        WHERE code_hash = $1
        "#,
    )
    .bind(&code_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::BadRequest("invalid invite code".into()))?;

    if row.revoked_at.is_some() {
        return Err(Error::BadRequest("invite revoked".into()));
    }
    if row.used_at.is_some() {
        return Err(Error::BadRequest("invite already used".into()));
    }
    if row.expires_at <= Utc::now() {
        return Err(Error::BadRequest("invite expired".into()));
    }
    if let Some(bound) = row.email.as_deref() {
        if bound != email.trim().to_ascii_lowercase() {
            return Err(Error::BadRequest("invite email mismatch".into()));
        }
    }
    Ok(())
}
