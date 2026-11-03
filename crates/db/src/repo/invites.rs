use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_token};
use linkdean_core::{
    CreateRegistrationInvite, CreateRegistrationInviteResponse, Error, RegistrationInvite,
    RegistrationInviteView, Result,
};
use sqlx::{PgPool, Postgres, Transaction};
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

/// Why an invite cannot be redeemed for `email`, if it cannot.
fn check_redeemable(row: &RegistrationInvite, email: &str) -> Result<()> {
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

/// Marks an invite used by `user_id`, inside the sign-up transaction that inserts that user.
/// `FOR UPDATE` makes a second sign-up with the same code wait for this one, then see it used;
/// if the sign-up fails, the rollback leaves the invite unused. Returns the invite id.
pub(crate) async fn claim_registration_invite(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
    email: &str,
    user_id: Uuid,
) -> Result<Uuid> {
    let code = code.trim();
    if code.is_empty() {
        return Err(Error::BadRequest("invite code required".into()));
    }
    let row = sqlx::query_as::<_, RegistrationInvite>(
        "SELECT * FROM registration_invites WHERE code_hash = $1 FOR UPDATE",
    )
    .bind(hash_token(code))
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::BadRequest("invalid invite code".into()))?;
    check_redeemable(&row, email)?;
    sqlx::query(
        "UPDATE registration_invites SET used_at = now(), used_by_user_id = $2 WHERE id = $1",
    )
    .bind(row.id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.id)
}
