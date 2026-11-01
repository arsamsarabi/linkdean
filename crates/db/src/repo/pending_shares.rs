use linkdean_core::{
    Error, PendingShareInvite, PendingShareInviteView, PendingShareSubjectType, Result,
    ShareAccess, UpdatePendingShareInvite,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::dashboard_shares::require_dashboard_owner;
use super::shares::require_category_owner;
use super::users::{find_user_by_email, find_user_by_id};

fn to_view(row: PendingShareInvite) -> Result<PendingShareInviteView> {
    Ok(PendingShareInviteView {
        id: row.id,
        subject_type: PendingShareSubjectType::parse(&row.subject_type)
            .map_err(Error::BadRequest)?,
        subject_id: row.subject_id,
        invitee_email: row.invitee_email,
        access: ShareAccess::parse(&row.access).map_err(Error::BadRequest)?,
        invited_by_user_id: row.invited_by_user_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn normalize_email(email: &str) -> Result<String> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("valid email required".into()));
    }
    Ok(email)
}

async fn assert_owner(
    pool: &PgPool,
    owner_user_id: Uuid,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
) -> Result<()> {
    match subject_type {
        PendingShareSubjectType::Category => {
            let cat = require_category_owner(pool, owner_user_id, subject_id).await?;
            if cat.is_vault {
                return Err(Error::BadRequest("cannot share vault categories".into()));
            }
        }
        PendingShareSubjectType::Dashboard => {
            require_dashboard_owner(pool, owner_user_id, subject_id).await?;
        }
    }
    Ok(())
}

pub async fn list_pending_for_subject(
    pool: &PgPool,
    owner_user_id: Uuid,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
) -> Result<Vec<PendingShareInviteView>> {
    assert_owner(pool, owner_user_id, subject_type, subject_id).await?;

    let rows = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT *
        FROM pending_share_invites
        WHERE subject_type = $1
          AND subject_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at ASC
        "#,
    )
    .bind(subject_type.as_str())
    .bind(subject_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter().map(to_view).collect()
}

pub async fn upsert_pending_share_invite(
    pool: &PgPool,
    owner_user_id: Uuid,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
    email: &str,
    access: ShareAccess,
) -> Result<PendingShareInviteView> {
    assert_owner(pool, owner_user_id, subject_type, subject_id).await?;

    let email = normalize_email(email)?;
    if let Some(owner) = find_user_by_id(pool, owner_user_id).await? {
        if owner.email.eq_ignore_ascii_case(&email) {
            return Err(Error::BadRequest("cannot share with yourself".into()));
        }
    }
    if let Some(invitee) = find_user_by_email(pool, &email).await? {
        super::blocks::assert_not_blocked(pool, owner_user_id, invitee.id).await?;
    }

    let row = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        INSERT INTO pending_share_invites
            (subject_type, subject_id, invitee_email, access, invited_by_user_id)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (subject_type, subject_id, invitee_email)
            WHERE claimed_at IS NULL AND revoked_at IS NULL
        DO UPDATE SET
            access = EXCLUDED.access,
            invited_by_user_id = EXCLUDED.invited_by_user_id,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(&email)
    .bind(access.as_str())
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    to_view(row)
}

pub async fn update_pending_share_invite(
    pool: &PgPool,
    owner_user_id: Uuid,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
    pending_id: Uuid,
    req: UpdatePendingShareInvite,
) -> Result<PendingShareInviteView> {
    assert_owner(pool, owner_user_id, subject_type, subject_id).await?;

    let row = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        UPDATE pending_share_invites
        SET access = $1, updated_at = now()
        WHERE id = $2
          AND subject_type = $3
          AND subject_id = $4
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        RETURNING *
        "#,
    )
    .bind(req.access.as_str())
    .bind(pending_id)
    .bind(subject_type.as_str())
    .bind(subject_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    to_view(row)
}

pub async fn revoke_pending_share_invite(
    pool: &PgPool,
    owner_user_id: Uuid,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
    pending_id: Uuid,
) -> Result<()> {
    assert_owner(pool, owner_user_id, subject_type, subject_id).await?;

    let res = sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET revoked_at = now(), updated_at = now()
        WHERE id = $1
          AND subject_type = $2
          AND subject_id = $3
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(pending_id)
    .bind(subject_type.as_str())
    .bind(subject_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// After register / OIDC signup: turn matching pending invites into real shares.
pub async fn claim_pending_share_invites(pool: &PgPool, email: &str, user_id: Uuid) -> Result<u32> {
    let email = normalize_email(email)?;
    let rows = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT *
        FROM pending_share_invites
        WHERE invitee_email = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at ASC
        "#,
    )
    .bind(&email)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut claimed = 0u32;
    for row in rows {
        if super::blocks::is_blocked_either_way(pool, user_id, row.invited_by_user_id).await? {
            // Revoke without applying so invite cannot linger.
            sqlx::query(
                r#"
                UPDATE pending_share_invites
                SET revoked_at = now(), updated_at = now()
                WHERE id = $1
                  AND claimed_at IS NULL
                  AND revoked_at IS NULL
                "#,
            )
            .bind(row.id)
            .execute(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            continue;
        }
        let Ok(subject_type) = PendingShareSubjectType::parse(&row.subject_type) else {
            continue;
        };
        let access = row.access.clone();

        let applied = match subject_type {
            PendingShareSubjectType::Category => {
                apply_category_claim(pool, &row, user_id, &access).await?
            }
            PendingShareSubjectType::Dashboard => {
                apply_dashboard_claim(pool, &row, user_id, &access).await?
            }
        };

        sqlx::query(
            r#"
            UPDATE pending_share_invites
            SET claimed_at = now(),
                claimed_by_user_id = $2,
                updated_at = now()
            WHERE id = $1
              AND claimed_at IS NULL
              AND revoked_at IS NULL
            "#,
        )
        .bind(row.id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        if applied {
            claimed += 1;
        }
    }
    Ok(claimed)
}

async fn apply_category_claim(
    pool: &PgPool,
    row: &PendingShareInvite,
    invitee_user_id: Uuid,
    access: &str,
) -> Result<bool> {
    let cat = match require_category_owner(pool, row.invited_by_user_id, row.subject_id).await {
        Ok(c) => c,
        Err(Error::NotFound | Error::Forbidden) => return Ok(false),
        Err(e) => return Err(e),
    };
    if cat.is_vault || cat.deleted_at.is_some() {
        return Ok(false);
    }
    if invitee_user_id == row.invited_by_user_id {
        return Ok(false);
    }

    let res = sqlx::query(
        r#"
        INSERT INTO category_shares (category_id, invitee_user_id, access, invited_by_user_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (category_id, invitee_user_id) DO UPDATE
          SET access = EXCLUDED.access, updated_at = now()
        "#,
    )
    .bind(row.subject_id)
    .bind(invitee_user_id)
    .bind(access)
    .bind(row.invited_by_user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if res.rows_affected() > 0 {
        if let Ok(access) = ShareAccess::parse(access) {
            super::notifications::notify_category_share_invite(
                pool,
                row.invited_by_user_id,
                invitee_user_id,
                row.subject_id,
                &cat.name,
                access,
            )
            .await;
        }
        return Ok(true);
    }
    Ok(false)
}

async fn apply_dashboard_claim(
    pool: &PgPool,
    row: &PendingShareInvite,
    invitee_user_id: Uuid,
    access: &str,
) -> Result<bool> {
    let dash = match require_dashboard_owner(pool, row.invited_by_user_id, row.subject_id).await {
        Ok(d) => d,
        Err(Error::NotFound | Error::Forbidden) => return Ok(false),
        Err(e) => return Err(e),
    };
    if dash.deleted_at.is_some() {
        return Ok(false);
    }
    if invitee_user_id == row.invited_by_user_id {
        return Ok(false);
    }

    let res = sqlx::query(
        r#"
        INSERT INTO dashboard_shares (dashboard_id, invitee_user_id, access, invited_by_user_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (dashboard_id, invitee_user_id) DO UPDATE
          SET access = EXCLUDED.access, updated_at = now()
        "#,
    )
    .bind(row.subject_id)
    .bind(invitee_user_id)
    .bind(access)
    .bind(row.invited_by_user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if res.rows_affected() > 0 {
        if let Ok(access) = ShareAccess::parse(access) {
            super::notifications::notify_dashboard_share_invite(
                pool,
                row.invited_by_user_id,
                invitee_user_id,
                row.subject_id,
                &dash.title,
                access,
            )
            .await;
        }
        return Ok(true);
    }
    Ok(false)
}
