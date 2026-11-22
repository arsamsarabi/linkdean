//! Organization invite links (Phase B). Role, expiry, use limit; gated by org setting.

use chrono::{Duration, Utc};
use linkdean_core::crypto::{self, hash_token};
use linkdean_core::{
    AcceptOrganizationInviteLink, CreateOrganizationInviteLink,
    CreateOrganizationInviteLinkResponse, Error, OrganizationInviteLink,
    OrganizationInviteLinkView, OrganizationRole, OrganizationWorkspace,
    PatchOrganizationInviteLinks, Result, TeamRole, User,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::organizations::{
    add_organization_member, get_organization_for_member, require_org_role,
};

const TOKEN_PREFIX: &str = "ldoi_";

fn parse_org_role(s: &str) -> Result<OrganizationRole> {
    OrganizationRole::parse(s).map_err(|_| Error::msg(format!("invalid organization role `{s}`")))
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

async fn invite_links_enabled(pool: &PgPool, organization_id: Uuid) -> Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT invite_links_enabled FROM organizations
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

async fn team_ids_for_link(pool: &PgPool, link_id: Uuid) -> Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT team_id FROM organization_invite_link_teams
        WHERE link_id = $1
        ORDER BY team_id
        "#,
    )
    .bind(link_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

fn to_view(
    row: OrganizationInviteLink,
    organization_name: String,
    team_ids: Vec<Uuid>,
) -> Result<OrganizationInviteLinkView> {
    let status = if row.revoked_at.is_some() {
        "revoked"
    } else if row.expires_at <= Utc::now() {
        "expired"
    } else if row.max_uses.is_some_and(|max| row.use_count >= max) {
        "exhausted"
    } else {
        "pending"
    };
    Ok(OrganizationInviteLinkView {
        id: row.id,
        organization_id: row.organization_id,
        organization_name,
        code_prefix: row.code_prefix,
        role: parse_org_role(&row.role)?,
        created_by_user_id: row.created_by_user_id,
        expires_at: row.expires_at,
        max_uses: row.max_uses,
        use_count: row.use_count,
        team_ids,
        created_at: row.created_at,
        revoked_at: row.revoked_at,
        status: status.into(),
    })
}

pub async fn set_organization_invite_links_enabled(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    req: PatchOrganizationInviteLinks,
) -> Result<OrganizationWorkspace> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;

    let n = sqlx::query(
        r#"
        UPDATE organizations
        SET invite_links_enabled = $2, updated_at = now()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(organization_id)
    .bind(req.invite_links_enabled)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();
    if n == 0 {
        return Err(Error::NotFound);
    }

    sqlx::query(
        r#"
        INSERT INTO organization_audit_events (organization_id, actor_user_id, action, payload)
        VALUES ($1, $2, 'invite_links.setting', $3)
        "#,
    )
    .bind(organization_id)
    .bind(actor_id)
    .bind(json!({ "invite_links_enabled": req.invite_links_enabled }))
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    get_organization_for_member(pool, actor_id, organization_id).await
}

pub async fn list_organization_invite_links(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
) -> Result<Vec<OrganizationInviteLinkView>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let rows = sqlx::query_as::<_, OrganizationInviteLink>(
        r#"
        SELECT id, organization_id, code_hash, code_prefix, role, created_by_user_id,
               expires_at, max_uses, use_count, revoked_at, created_at, updated_at
        FROM organization_invite_links
        WHERE organization_id = $1
        ORDER BY created_at DESC
        LIMIT 200
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let name = org_name(pool, organization_id).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let teams = team_ids_for_link(pool, row.id).await?;
        out.push(to_view(row, name.clone(), teams)?);
    }
    Ok(out)
}

pub async fn create_organization_invite_link(
    pool: &PgPool,
    actor: &User,
    organization_id: Uuid,
    req: CreateOrganizationInviteLink,
) -> Result<CreateOrganizationInviteLinkResponse> {
    require_org_role(pool, actor.id, organization_id, OrganizationRole::Admin).await?;

    if !invite_links_enabled(pool, organization_id).await? {
        return Err(Error::BadRequest(
            "invite links are disabled for this organization".into(),
        ));
    }

    if matches!(req.role, OrganizationRole::Owner) {
        return Err(Error::BadRequest(
            "invite as admin, member, or guest; promote to owner after they join".into(),
        ));
    }

    if req.role == OrganizationRole::Guest && req.teams.is_empty() {
        return Err(Error::BadRequest(
            "guests need at least one team on the invite link".into(),
        ));
    }

    if let Some(max) = req.max_uses {
        if max <= 0 {
            return Err(Error::BadRequest("max_uses must be positive".into()));
        }
    }

    for spec in &req.teams {
        let ok: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM teams
                WHERE id = $1 AND organization_id = $2
                  AND archived_at IS NULL AND deleted_at IS NULL
            )
            "#,
        )
        .bind(spec.team_id)
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if !ok {
            return Err(Error::BadRequest("unknown team on invite link".into()));
        }
    }

    let days = req.expires_in_days.unwrap_or(7).clamp(1, 90);
    let expires_at = Utc::now() + Duration::days(days as i64);

    let secret = hex::encode(crypto::random_bytes(32));
    let code = format!("{TOKEN_PREFIX}{secret}");
    let code_hash = hash_token(&code);
    let code_prefix = format!("{TOKEN_PREFIX}{}", &secret[..8.min(secret.len())]);

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    let row = sqlx::query_as::<_, OrganizationInviteLink>(
        r#"
        INSERT INTO organization_invite_links (
            organization_id, code_hash, code_prefix, role, created_by_user_id,
            expires_at, max_uses
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, organization_id, code_hash, code_prefix, role, created_by_user_id,
                  expires_at, max_uses, use_count, revoked_at, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(&code_hash)
    .bind(&code_prefix)
    .bind(req.role.as_str())
    .bind(actor.id)
    .bind(expires_at)
    .bind(req.max_uses)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    for spec in &req.teams {
        let team_role = spec.role.unwrap_or(TeamRole::Editor);
        sqlx::query(
            r#"
            INSERT INTO organization_invite_link_teams (link_id, team_id, team_role)
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
        VALUES ($1, $2, 'invite_link.created', $3)
        "#,
    )
    .bind(organization_id)
    .bind(actor.id)
    .bind(json!({
        "link_id": row.id,
        "role": req.role.as_str(),
        "max_uses": req.max_uses,
        "expires_at": expires_at,
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    let team_ids = team_ids_for_link(pool, row.id).await?;
    let name = org_name(pool, organization_id).await?;
    Ok(CreateOrganizationInviteLinkResponse {
        link: to_view(row, name, team_ids)?,
        code,
    })
}

pub async fn revoke_organization_invite_link(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    link_id: Uuid,
) -> Result<()> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let n = sqlx::query(
        r#"
        UPDATE organization_invite_links
        SET revoked_at = now(), updated_at = now()
        WHERE id = $1
          AND organization_id = $2
          AND revoked_at IS NULL
        "#,
    )
    .bind(link_id)
    .bind(organization_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();
    if n == 0 {
        return Err(Error::NotFound);
    }

    sqlx::query(
        r#"
        INSERT INTO organization_audit_events (organization_id, actor_user_id, action, payload)
        VALUES ($1, $2, 'invite_link.revoked', $3)
        "#,
    )
    .bind(organization_id)
    .bind(actor_id)
    .bind(json!({ "link_id": link_id }))
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok(())
}

pub async fn accept_organization_invite_link(
    pool: &PgPool,
    user: &User,
    req: AcceptOrganizationInviteLink,
) -> Result<OrganizationWorkspace> {
    if user.email_verified_at.is_none() {
        return Err(Error::EmailUnverified);
    }
    let code = req.code.trim();
    if code.is_empty() {
        return Err(Error::BadRequest("invite link code required".into()));
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    let row = sqlx::query_as::<_, OrganizationInviteLink>(
        r#"
        SELECT id, organization_id, code_hash, code_prefix, role, created_by_user_id,
               expires_at, max_uses, use_count, revoked_at, created_at, updated_at
        FROM organization_invite_links
        WHERE code_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(hash_token(code))
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or_else(|| Error::BadRequest("invalid invite link".into()))?;

    if row.revoked_at.is_some() {
        return Err(Error::BadRequest("invite link revoked".into()));
    }
    if row.expires_at <= Utc::now() {
        return Err(Error::BadRequest("invite link expired".into()));
    }
    if row.max_uses.is_some_and(|max| row.use_count >= max) {
        return Err(Error::BadRequest("invite link exhausted".into()));
    }

    let enabled: bool = sqlx::query_scalar(
        r#"
        SELECT invite_links_enabled FROM organizations
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(row.organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
    if !enabled {
        return Err(Error::BadRequest(
            "invite links are disabled for this organization".into(),
        ));
    }

    // Drop the lock before membership helpers that open their own connections/txs.
    let link_id = row.id;
    let organization_id = row.organization_id;
    let role = parse_org_role(&row.role)?;
    let team_specs: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT team_id, team_role FROM organization_invite_link_teams
        WHERE link_id = $1
        "#,
    )
    .bind(link_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Check membership inside the locked tx via a plain query.
    let already: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT m.role
        FROM organization_members m
        WHERE m.organization_id = $1 AND m.user_id = $2
        "#,
    )
    .bind(organization_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if already.is_some() {
        return Err(Error::Conflict("already a member".into()));
    }

    sqlx::query(
        r#"
        UPDATE organization_invite_links
        SET use_count = use_count + 1, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(link_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO organization_audit_events (organization_id, actor_user_id, action, payload)
        VALUES ($1, $2, 'invite_link.accepted', $3)
        "#,
    )
    .bind(organization_id)
    .bind(user.id)
    .bind(json!({ "link_id": link_id, "role": role.as_str() }))
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    match add_organization_member(pool, organization_id, user.id, role).await {
        Ok(_) => {}
        Err(Error::Conflict(_)) => {
            // Race: another accept won; leave use_count incremented (ponytail: rare double-count).
            return Err(Error::Conflict("already a member".into()));
        }
        Err(e) => return Err(e),
    }

    for (team_id, team_role) in team_specs {
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

    get_organization_for_member(pool, user.id, organization_id).await
}
