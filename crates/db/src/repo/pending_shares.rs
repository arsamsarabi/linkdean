use linkdean_core::{
    Error, PendingShareInvite, PendingShareInviteView, PendingShareSubjectType, Result,
    ShareAccess, ShareInviteInboxItem, UpdatePendingShareInvite,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::dashboard_shares::require_dashboard_owner;
use super::shares::require_category_owner;
use super::users::{find_user_by_email, find_user_by_id, user_share_label};

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
          AND rejected_at IS NULL
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
    invitee_user_id: Option<Uuid>,
) -> Result<(PendingShareInviteView, bool)> {
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

    if let Some(existing) =
        find_open_invite(pool, subject_type, subject_id, &email, invitee_user_id).await?
    {
        if existing.rejected_at.is_some() {
            return Err(Error::Conflict("This invite can't be sent.".into()));
        }
        let row = sqlx::query_as::<_, PendingShareInvite>(
            "UPDATE pending_share_invites SET access = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(access.as_str())
        .bind(existing.id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        return Ok((to_view(row)?, false));
    }

    let row = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        INSERT INTO pending_share_invites
            (subject_type, subject_id, invitee_email, access, invited_by_user_id, invitee_user_id)
        VALUES ($1, $2, $3, $4, $5, $6)
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
    .bind(invitee_user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok((to_view(row)?, true))
}

async fn find_open_invite(
    pool: &PgPool,
    subject_type: PendingShareSubjectType,
    subject_id: Uuid,
    email: &str,
    invitee_user_id: Option<Uuid>,
) -> Result<Option<PendingShareInvite>> {
    sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE subject_type = $1
          AND subject_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
          AND (invitee_email = $3 OR invitee_user_id = $4)
        ORDER BY rejected_at NULLS LAST
        LIMIT 1
        "#,
    )
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(email)
    .bind(invitee_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
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
          AND rejected_at IS NULL
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
          AND rejected_at IS NULL
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

/// After register / OIDC signup: attach invites sent to this email. Access still needs acceptance.
pub async fn link_pending_share_invites(pool: &PgPool, email: &str, user_id: Uuid) -> Result<u32> {
    let email = normalize_email(email)?;
    let rows = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
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

    let mut linked = 0u32;
    for row in rows {
        if super::blocks::is_blocked_either_way(pool, user_id, row.invited_by_user_id).await? {
            revoke_invite(pool, row.id).await?;
            continue;
        }
        sqlx::query(
            "UPDATE pending_share_invites SET invitee_user_id = $2, updated_at = now() WHERE id = $1",
        )
        .bind(row.id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        notify_invitee(pool, &row, user_id).await?;
        linked += 1;
    }
    Ok(linked)
}

async fn revoke_invite(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET revoked_at = now(), updated_at = now()
        WHERE id = $1 AND claimed_at IS NULL AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Title of the shared subject, or `None` when it can no longer be shared.
async fn invite_subject_title(pool: &PgPool, row: &PendingShareInvite) -> Result<Option<String>> {
    match PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)? {
        PendingShareSubjectType::Category => {
            match require_category_owner(pool, row.invited_by_user_id, row.subject_id).await {
                Ok(c) if !c.is_vault && c.deleted_at.is_none() => Ok(Some(c.name)),
                Ok(_) | Err(Error::NotFound | Error::Forbidden) => Ok(None),
                Err(e) => Err(e),
            }
        }
        PendingShareSubjectType::Dashboard => {
            match require_dashboard_owner(pool, row.invited_by_user_id, row.subject_id).await {
                Ok(d) if d.deleted_at.is_none() => Ok(Some(d.title)),
                Ok(_) | Err(Error::NotFound | Error::Forbidden) => Ok(None),
                Err(e) => Err(e),
            }
        }
    }
}

async fn notify_invitee(
    pool: &PgPool,
    row: &PendingShareInvite,
    invitee_user_id: Uuid,
) -> Result<()> {
    let Some(title) = invite_subject_title(pool, row).await? else {
        return Ok(());
    };
    let access = ShareAccess::parse(&row.access).map_err(Error::BadRequest)?;
    match PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)? {
        PendingShareSubjectType::Category => {
            super::notifications::notify_category_share_invite(
                pool,
                row.invited_by_user_id,
                invitee_user_id,
                row.subject_id,
                &title,
                access,
            )
            .await;
        }
        PendingShareSubjectType::Dashboard => {
            super::notifications::notify_dashboard_share_invite(
                pool,
                row.invited_by_user_id,
                invitee_user_id,
                row.subject_id,
                &title,
                access,
            )
            .await;
        }
    }
    Ok(())
}

async fn find_invitee_row(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<PendingShareInvite> {
    sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE id = $1
          AND invitee_user_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

pub async fn list_share_invites(pool: &PgPool, user_id: Uuid) -> Result<Vec<ShareInviteInboxItem>> {
    let rows = sqlx::query_as::<_, PendingShareInvite>(
        r#"
        SELECT * FROM pending_share_invites
        WHERE invitee_user_id = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(subject_title) = invite_subject_title(pool, &row).await? else {
            continue;
        };
        let inviter_display_name = match find_user_by_id(pool, row.invited_by_user_id).await? {
            Some(u) => user_share_label(&u),
            None => "User".into(),
        };
        out.push(ShareInviteInboxItem {
            id: row.id,
            subject_type: PendingShareSubjectType::parse(&row.subject_type)
                .map_err(Error::BadRequest)?,
            subject_id: row.subject_id,
            subject_title,
            access: ShareAccess::parse(&row.access).map_err(Error::BadRequest)?,
            inviter_display_name,
            created_at: row.created_at,
            rejected_at: row.rejected_at,
        });
    }
    Ok(out)
}

pub async fn accept_share_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = find_invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_some() {
        return Err(Error::NotFound);
    }
    if super::blocks::is_blocked_either_way(pool, user_id, row.invited_by_user_id).await? {
        revoke_invite(pool, row.id).await?;
        return Err(Error::NotFound);
    }

    // Claim first: a double click or an owner revoke then loses the race cleanly.
    let claimed = sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET claimed_at = now(), claimed_by_user_id = $2, updated_at = now()
        WHERE id = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
          AND rejected_at IS NULL
        "#,
    )
    .bind(row.id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if claimed.rows_affected() == 0 {
        return Err(Error::NotFound);
    }

    let subject = PendingShareSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)?;
    let applied = match subject {
        PendingShareSubjectType::Category => {
            apply_category_claim(pool, &row, user_id, &row.access).await
        }
        PendingShareSubjectType::Dashboard => {
            apply_dashboard_claim(pool, &row, user_id, &row.access).await
        }
    };
    match applied {
        Ok(true) => Ok(()),
        Ok(false) => {
            release_claim(pool, row.id, true).await?;
            Err(Error::NotFound)
        }
        Err(e) => {
            release_claim(pool, row.id, false).await?;
            Err(e)
        }
    }
}

async fn release_claim(pool: &PgPool, id: Uuid, revoke: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET claimed_at = NULL,
            claimed_by_user_id = NULL,
            revoked_at = CASE WHEN $2 THEN now() ELSE revoked_at END,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(revoke)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn reject_share_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = find_invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_some() {
        return Err(Error::NotFound);
    }
    set_rejected(pool, row.id, true).await
}

pub async fn restore_share_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = find_invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_none() {
        return Err(Error::NotFound);
    }
    set_rejected(pool, row.id, false).await
}

async fn set_rejected(pool: &PgPool, id: Uuid, rejected: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET rejected_at = CASE WHEN $2 THEN now() END, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(rejected)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
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

    Ok(res.rows_affected() > 0)
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

    Ok(res.rows_affected() > 0)
}
