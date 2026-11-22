//! Organization email invites. Access starts only when the invitee accepts.

use linkdean_core::{
    CreateOrganizationInvite, Error, OrganizationInvite, OrganizationInviteInboxItem,
    OrganizationInviteView, OrganizationRole, Result, TeamRole, User,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::organizations::{add_organization_member, membership_role, require_org_role};
use super::users::{find_user_by_email, find_user_by_id, user_share_label};

fn normalize_email(email: &str) -> Result<String> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("valid email required".into()));
    }
    Ok(email)
}

fn parse_org_role(s: &str) -> Result<OrganizationRole> {
    OrganizationRole::parse(s).map_err(|_| Error::msg(format!("invalid organization role `{s}`")))
}

async fn team_ids_for_invite(pool: &PgPool, invite_id: Uuid) -> Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT team_id FROM organization_invite_teams
        WHERE invite_id = $1
        ORDER BY team_id
        "#,
    )
    .bind(invite_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

async fn org_name(pool: &PgPool, organization_id: Uuid) -> Result<String> {
    sqlx::query_scalar(
        r#"
        SELECT name FROM organizations
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

async fn to_view(pool: &PgPool, row: OrganizationInvite) -> Result<OrganizationInviteView> {
    Ok(OrganizationInviteView {
        id: row.id,
        organization_id: row.organization_id,
        organization_name: org_name(pool, row.organization_id).await?,
        invitee_email: row.invitee_email,
        role: parse_org_role(&row.role)?,
        invited_by_user_id: row.invited_by_user_id,
        invitee_user_id: row.invitee_user_id,
        team_ids: team_ids_for_invite(pool, row.id).await?,
        created_at: row.created_at,
        rejected_at: row.rejected_at,
    })
}

/// Create or refresh an open invite. Returns (view, newly_notifiable) when a verified
/// account is attached and should get an inbox ping.
pub async fn create_organization_invite(
    pool: &PgPool,
    actor: &User,
    organization_id: Uuid,
    req: CreateOrganizationInvite,
) -> Result<(OrganizationInviteView, Option<Uuid>)> {
    require_org_role(pool, actor.id, organization_id, OrganizationRole::Admin).await?;

    if matches!(req.role, OrganizationRole::Owner) {
        return Err(Error::BadRequest(
            "invite as admin, member, or guest; promote to owner after they join".into(),
        ));
    }

    let email = normalize_email(&req.email)?;
    if actor.email.eq_ignore_ascii_case(&email) {
        return Err(Error::BadRequest("cannot invite yourself".into()));
    }

    if req.role == OrganizationRole::Guest && req.teams.is_empty() {
        return Err(Error::BadRequest(
            "guests need at least one team on the invite".into(),
        ));
    }

    // Validate teams belong to this org.
    for spec in &req.teams {
        let ok: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM teams
                WHERE id = $1 AND organization_id = $2 AND archived_at IS NULL
            )
            "#,
        )
        .bind(spec.team_id)
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if !ok {
            return Err(Error::BadRequest("unknown team on invite".into()));
        }
    }

    let existing_user = find_user_by_email(pool, &email).await?;
    if let Some(ref u) = existing_user {
        if membership_role(pool, u.id, organization_id)
            .await?
            .is_some()
        {
            return Err(Error::Conflict("already a member".into()));
        }
        if let Some(other) = find_open_invite_for_user(pool, organization_id, u.id).await? {
            if other.rejected_at.is_some() {
                return Err(Error::Conflict("This invite can't be sent.".into()));
            }
        }
    }

    // Attach only when verified (same rule as folder shares).
    let invitee_user_id = existing_user
        .as_ref()
        .filter(|u| u.email_verified_at.is_some())
        .map(|u| u.id);

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    // Revoke any prior open invite for this email so unique index allows the new row.
    sqlx::query(
        r#"
        UPDATE organization_invites
        SET revoked_at = now(), updated_at = now()
        WHERE organization_id = $1
          AND lower(invitee_email) = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(organization_id)
    .bind(&email)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let row = sqlx::query_as::<_, OrganizationInvite>(
        r#"
        INSERT INTO organization_invites (
            organization_id, invitee_email, role, invited_by_user_id, invitee_user_id
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, organization_id, invitee_email, role, invited_by_user_id, invitee_user_id,
                  created_at, updated_at, claimed_at, claimed_by_user_id, revoked_at, rejected_at
        "#,
    )
    .bind(organization_id)
    .bind(&email)
    .bind(req.role.as_str())
    .bind(actor.id)
    .bind(invitee_user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    for spec in &req.teams {
        let team_role = spec.role.unwrap_or(TeamRole::Editor);
        sqlx::query(
            r#"
            INSERT INTO organization_invite_teams (invite_id, team_id, team_role)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(row.id)
        .bind(spec.team_id)
        .bind(team_role.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }

    sqlx::query(
        r#"
        INSERT INTO organization_audit_events (organization_id, actor_user_id, action, payload)
        VALUES ($1, $2, 'invite.created', $3)
        "#,
    )
    .bind(organization_id)
    .bind(actor.id)
    .bind(json!({ "invite_id": row.id, "email": email, "role": req.role.as_str() }))
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    let view = to_view(pool, row).await?;
    Ok((view, invitee_user_id))
}

async fn find_open_invite_for_user(
    pool: &PgPool,
    organization_id: Uuid,
    user_id: Uuid,
) -> Result<Option<OrganizationInvite>> {
    sqlx::query_as::<_, OrganizationInvite>(
        r#"
        SELECT id, organization_id, invitee_email, role, invited_by_user_id, invitee_user_id,
               created_at, updated_at, claimed_at, claimed_by_user_id, revoked_at, rejected_at
        FROM organization_invites
        WHERE organization_id = $1
          AND invitee_user_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn list_organization_invites(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
) -> Result<Vec<OrganizationInviteView>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let rows = sqlx::query_as::<_, OrganizationInvite>(
        r#"
        SELECT id, organization_id, invitee_email, role, invited_by_user_id, invitee_user_id,
               created_at, updated_at, claimed_at, claimed_by_user_id, revoked_at, rejected_at
        FROM organization_invites
        WHERE organization_id = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        ORDER BY created_at ASC
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(to_view(pool, row).await?);
    }
    Ok(out)
}

pub async fn revoke_organization_invite(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    invite_id: Uuid,
) -> Result<()> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let n = sqlx::query(
        r#"
        UPDATE organization_invites
        SET revoked_at = now(), updated_at = now()
        WHERE id = $1
          AND organization_id = $2
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(invite_id)
    .bind(organization_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();
    if n == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn list_organization_invite_inbox(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<OrganizationInviteInboxItem>> {
    let rows = sqlx::query_as::<_, OrganizationInvite>(
        r#"
        SELECT i.id, i.organization_id, i.invitee_email, i.role, i.invited_by_user_id,
               i.invitee_user_id, i.created_at, i.updated_at, i.claimed_at,
               i.claimed_by_user_id, i.revoked_at, i.rejected_at
        FROM organization_invites i
        INNER JOIN organizations o ON o.id = i.organization_id
        WHERE i.invitee_user_id = $1
          AND i.claimed_at IS NULL
          AND i.revoked_at IS NULL
          AND o.deleted_at IS NULL
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let inviter = find_user_by_id(pool, row.invited_by_user_id)
            .await?
            .map(|u| user_share_label(&u))
            .unwrap_or_else(|| "User".into());
        out.push(OrganizationInviteInboxItem {
            id: row.id,
            organization_id: row.organization_id,
            organization_name: org_name(pool, row.organization_id).await?,
            role: parse_org_role(&row.role)?,
            inviter_display_name: inviter,
            created_at: row.created_at,
            rejected_at: row.rejected_at,
        });
    }
    Ok(out)
}

async fn invitee_row(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<OrganizationInvite> {
    sqlx::query_as::<_, OrganizationInvite>(
        r#"
        SELECT id, organization_id, invitee_email, role, invited_by_user_id, invitee_user_id,
               created_at, updated_at, claimed_at, claimed_by_user_id, revoked_at, rejected_at
        FROM organization_invites
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

pub async fn accept_organization_invite(pool: &PgPool, user: &User, id: Uuid) -> Result<()> {
    if user.email_verified_at.is_none() {
        return Err(Error::EmailUnverified);
    }
    let row = invitee_row(pool, user.id, id).await?;
    if row.rejected_at.is_some() {
        return Err(Error::NotFound);
    }
    if membership_role(pool, user.id, row.organization_id)
        .await?
        .is_some()
    {
        // Already a member — claim the invite so it leaves the inbox.
        let _ = claim_invite(pool, row.id, user.id).await?;
        return Ok(());
    }

    let role = parse_org_role(&row.role)?;
    let claimed = claim_invite(pool, row.id, user.id).await?;
    if !claimed {
        return Err(Error::NotFound);
    }

    match add_organization_member(pool, row.organization_id, user.id, role).await {
        Ok(_) => {}
        Err(Error::Conflict(_)) => {}
        Err(e) => {
            unclaim_invite(pool, row.id).await?;
            return Err(e);
        }
    }

    // Apply explicit team assignments (and for guests, only those — not default).
    let specs: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT team_id, team_role FROM organization_invite_teams
        WHERE invite_id = $1
        "#,
    )
    .bind(row.id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    for (team_id, team_role) in specs {
        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (team_id, user_id) DO UPDATE SET role = EXCLUDED.role, updated_at = now()
            "#,
        )
        .bind(team_id)
        .bind(user.id)
        .bind(&team_role)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }

    if role == OrganizationRole::Guest {
        // Guests must not stay on the default team from add_organization_member (it skips them).
        // Nothing to strip.
    }

    Ok(())
}

async fn claim_invite(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool> {
    let n = sqlx::query(
        r#"
        UPDATE organization_invites
        SET claimed_at = now(), claimed_by_user_id = $2, updated_at = now()
        WHERE id = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
          AND rejected_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();
    Ok(n > 0)
}

async fn unclaim_invite(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE organization_invites
        SET claimed_at = NULL, claimed_by_user_id = NULL, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn reject_organization_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let row = invitee_row(pool, user_id, id).await?;
    if row.rejected_at.is_some() {
        return Err(Error::NotFound);
    }
    let n = sqlx::query(
        r#"
        UPDATE organization_invites
        SET rejected_at = now(), updated_at = now()
        WHERE id = $1 AND rejected_at IS NULL AND claimed_at IS NULL AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();
    if n == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn restore_organization_invite(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let n = sqlx::query(
        r#"
        UPDATE organization_invites
        SET rejected_at = NULL, updated_at = now()
        WHERE id = $1
          AND invitee_user_id = $2
          AND rejected_at IS NOT NULL
          AND claimed_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();
    if n == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// Attach open invites for this email to the user once the address is verified.
pub async fn link_pending_organization_invites(
    pool: &PgPool,
    email: &str,
    user_id: Uuid,
) -> Result<()> {
    let email = normalize_email(email)?;
    sqlx::query(
        r#"
        UPDATE organization_invites
        SET invitee_user_id = $2, updated_at = now()
        WHERE lower(invitee_email) = $1
          AND claimed_at IS NULL
          AND revoked_at IS NULL
          AND invitee_user_id IS NULL
        "#,
    )
    .bind(&email)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}
