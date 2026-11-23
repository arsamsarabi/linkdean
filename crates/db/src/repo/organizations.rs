//! Organization and team membership.

use linkdean_core::{
    CreateOrganization, CreateTeam, Error, Organization, OrganizationMember,
    OrganizationMemberView, OrganizationRole, OrganizationWorkspace, Result, Team, TeamRole,
    TeamView, User,
};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

const MAX_NAME: usize = 80;
const DEFAULT_TEAM_NAME: &str = "General";

fn normalize_name(raw: &str) -> Result<String> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(Error::BadRequest("name required".into()));
    }
    if name.len() > MAX_NAME {
        return Err(Error::BadRequest(format!("name max {MAX_NAME} chars")));
    }
    Ok(name)
}

fn parse_org_role(s: &str) -> Result<OrganizationRole> {
    OrganizationRole::parse(s).map_err(|_| Error::msg(format!("invalid organization role `{s}`")))
}

fn parse_team_role(s: &str) -> Result<TeamRole> {
    TeamRole::parse(s).map_err(|_| Error::msg(format!("invalid team role `{s}`")))
}

pub(crate) async fn record_audit(
    tx: &mut Transaction<'_, Postgres>,
    organization_id: Uuid,
    actor_user_id: Option<Uuid>,
    action: &str,
    payload: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO organization_audit_events (organization_id, actor_user_id, action, payload)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(organization_id)
    .bind(actor_user_id)
    .bind(action)
    .bind(payload)
    .execute(&mut **tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Create an organization, make `creator` the owner, and add the default team (Editor).
pub async fn create_organization(
    pool: &PgPool,
    creator: &User,
    req: CreateOrganization,
) -> Result<OrganizationWorkspace> {
    let name = normalize_name(&req.name)?;
    let settings = super::platform::get_platform_settings(pool).await?;
    if !settings.organizations_create_open && !creator.is_admin {
        return Err(Error::Forbidden);
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    let org = sqlx::query_as::<_, Organization>(
        r#"
        INSERT INTO organizations (name)
        VALUES ($1)
        RETURNING id, name, created_at, updated_at, deleted_at, invite_links_enabled
        "#,
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role)
        VALUES ($1, $2, 'owner')
        "#,
    )
    .bind(org.id)
    .bind(creator.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let team = sqlx::query_as::<_, Team>(
        r#"
        INSERT INTO teams (organization_id, name, is_default)
        VALUES ($1, $2, TRUE)
        RETURNING id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        "#,
    )
    .bind(org.id)
    .bind(DEFAULT_TEAM_NAME)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO team_members (team_id, user_id, role)
        VALUES ($1, $2, 'editor')
        "#,
    )
    .bind(team.id)
    .bind(creator.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    record_audit(
        &mut tx,
        org.id,
        Some(creator.id),
        "organization.created",
        json!({ "name": name }),
    )
    .await?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    Ok(OrganizationWorkspace {
        id: org.id,
        name: org.name,
        role: OrganizationRole::Owner,
        deleted_at: None,
        invite_links_enabled: org.invite_links_enabled,
        created_at: org.created_at,
        updated_at: org.updated_at,
    })
}

pub async fn list_organizations_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<OrganizationWorkspace>> {
    // Live orgs for every member; soft-deleted orgs only for owners (grace restore).
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            bool,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT o.id, o.name, m.role, o.deleted_at, o.invite_links_enabled, o.created_at, o.updated_at
        FROM organizations o
        INNER JOIN organization_members m ON m.organization_id = o.id
        WHERE m.user_id = $1
          AND (
            o.deleted_at IS NULL
            OR m.role = 'owner'
          )
        ORDER BY o.deleted_at NULLS FIRST, lower(o.name), o.id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter()
        .map(
            |(id, name, role, deleted_at, invite_links_enabled, created_at, updated_at)| {
                Ok(OrganizationWorkspace {
                    id,
                    name,
                    role: parse_org_role(&role)?,
                    deleted_at,
                    invite_links_enabled,
                    created_at,
                    updated_at,
                })
            },
        )
        .collect()
}

pub async fn get_organization_for_member(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<OrganizationWorkspace> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            bool,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT o.id, o.name, m.role, o.deleted_at, o.invite_links_enabled, o.created_at, o.updated_at
        FROM organizations o
        INNER JOIN organization_members m ON m.organization_id = o.id
        WHERE o.id = $1
          AND m.user_id = $2
          AND (
            o.deleted_at IS NULL
            OR m.role = 'owner'
          )
        "#,
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    Ok(OrganizationWorkspace {
        id: row.0,
        name: row.1,
        role: parse_org_role(&row.2)?,
        deleted_at: row.3,
        invite_links_enabled: row.4,
        created_at: row.5,
        updated_at: row.6,
    })
}

pub async fn require_org_role(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
    min: OrganizationRole,
) -> Result<OrganizationRole> {
    let role = membership_role(pool, user_id, organization_id)
        .await?
        .ok_or(Error::NotFound)?;
    if !role.at_least(min) {
        return Err(Error::Forbidden);
    }
    Ok(role)
}

pub async fn membership_role(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<Option<OrganizationRole>> {
    let role: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT m.role
        FROM organization_members m
        INNER JOIN organizations o ON o.id = m.organization_id
        WHERE m.organization_id = $1
          AND m.user_id = $2
          AND o.deleted_at IS NULL
        "#,
    )
    .bind(organization_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    role.map(|(s,)| parse_org_role(&s)).transpose()
}

/// Test / invite helpers: add a member (and default-team Editor unless guest).
pub async fn add_organization_member(
    pool: &PgPool,
    organization_id: Uuid,
    user_id: Uuid,
    role: OrganizationRole,
) -> Result<OrganizationMember> {
    if matches!(role, OrganizationRole::Owner) {
        // Owners are promoted via patch; create path starts with one owner.
        return Err(Error::BadRequest(
            "add as admin/member/guest, then promote to owner".into(),
        ));
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    let member = sqlx::query_as::<_, OrganizationMember>(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING organization_id, user_id, role, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(user_id)
    .bind(role.as_str())
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("organization_members_pkey") {
            Error::Conflict("already a member".into())
        } else {
            Error::msg(msg)
        }
    })?;

    if role != OrganizationRole::Guest {
        let default_team_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT id FROM teams
            WHERE organization_id = $1 AND is_default
            "#,
        )
        .bind(organization_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, 'editor')
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(default_team_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }

    record_audit(
        &mut tx,
        organization_id,
        None,
        "member.added",
        json!({ "user_id": user_id, "role": role.as_str() }),
    )
    .await?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(member)
}

pub async fn list_organization_members(
    pool: &PgPool,
    viewer_id: Uuid,
    organization_id: Uuid,
) -> Result<Vec<OrganizationMemberView>> {
    let role = require_org_role(pool, viewer_id, organization_id, OrganizationRole::Guest).await?;
    if role == OrganizationRole::Guest {
        return Err(Error::Forbidden);
    }

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT m.user_id, m.role, u.email, u.display_name, m.created_at
        FROM organization_members m
        INNER JOIN users u ON u.id = m.user_id
        WHERE m.organization_id = $1
        ORDER BY
            CASE m.role
                WHEN 'owner' THEN 0
                WHEN 'admin' THEN 1
                WHEN 'member' THEN 2
                ELSE 3
            END,
            lower(coalesce(u.display_name, u.email)),
            m.user_id
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter()
        .map(|(user_id, role, email, display_name, created_at)| {
            Ok(OrganizationMemberView {
                user_id,
                role: parse_org_role(&role)?,
                email,
                display_name,
                created_at,
            })
        })
        .collect()
}

pub async fn patch_organization_member(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    target_user_id: Uuid,
    new_role: OrganizationRole,
) -> Result<OrganizationMemberView> {
    let actor_role =
        require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;

    let current = membership_role(pool, target_user_id, organization_id)
        .await?
        .ok_or(Error::NotFound)?;

    if current == OrganizationRole::Owner || new_role == OrganizationRole::Owner {
        if actor_role != OrganizationRole::Owner {
            return Err(Error::Forbidden);
        }
    }

    if current == OrganizationRole::Owner && new_role != OrganizationRole::Owner {
        let owners: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)::bigint FROM organization_members
            WHERE organization_id = $1 AND role = 'owner'
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if owners <= 1 {
            return Err(Error::BadRequest(
                "promote another owner before demoting the last owner".into(),
            ));
        }
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        UPDATE organization_members
        SET role = $3, updated_at = now()
        WHERE organization_id = $1 AND user_id = $2
        "#,
    )
    .bind(organization_id)
    .bind(target_user_id)
    .bind(new_role.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Guests leave the default team; non-guests join it as Editor if missing.
    let default_team_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM teams
        WHERE organization_id = $1 AND is_default
        "#,
    )
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if let Some(team_id) = default_team_id {
        if new_role == OrganizationRole::Guest {
            sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
                .bind(team_id)
                .bind(target_user_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::msg(e.to_string()))?;
        } else if current == OrganizationRole::Guest {
            sqlx::query(
                r#"
                INSERT INTO team_members (team_id, user_id, role)
                VALUES ($1, $2, 'editor')
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(team_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        }
    }

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "member.role_changed",
        json!({
            "user_id": target_user_id,
            "from": current.as_str(),
            "to": new_role.as_str()
        }),
    )
    .await?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    list_organization_members(pool, actor_id, organization_id)
        .await?
        .into_iter()
        .find(|m| m.user_id == target_user_id)
        .ok_or(Error::NotFound)
}

pub async fn remove_organization_member(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    target_user_id: Uuid,
) -> Result<()> {
    let actor_role =
        require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let current = membership_role(pool, target_user_id, organization_id)
        .await?
        .ok_or(Error::NotFound)?;

    if current == OrganizationRole::Owner && actor_role != OrganizationRole::Owner {
        return Err(Error::Forbidden);
    }
    if current == OrganizationRole::Owner {
        let owners: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)::bigint FROM organization_members
            WHERE organization_id = $1 AND role = 'owner'
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if owners <= 1 {
            return Err(Error::BadRequest(
                "promote another owner before removing the last owner".into(),
            ));
        }
    }

    leave_or_remove(
        pool,
        Some(actor_id),
        organization_id,
        target_user_id,
        "member.removed",
    )
    .await
}

pub async fn leave_organization(pool: &PgPool, user_id: Uuid, organization_id: Uuid) -> Result<()> {
    let role = membership_role(pool, user_id, organization_id)
        .await?
        .ok_or(Error::NotFound)?;
    if role == OrganizationRole::Owner {
        let owners: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)::bigint FROM organization_members
            WHERE organization_id = $1 AND role = 'owner'
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if owners <= 1 {
            return Err(Error::BadRequest(
                "promote another owner before leaving".into(),
            ));
        }
    }
    leave_or_remove(pool, Some(user_id), organization_id, user_id, "member.left").await
}

async fn leave_or_remove(
    pool: &PgPool,
    actor_id: Option<Uuid>,
    organization_id: Uuid,
    target_user_id: Uuid,
    action: &str,
) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        DELETE FROM team_members tm
        USING teams t
        WHERE tm.team_id = t.id
          AND t.organization_id = $1
          AND tm.user_id = $2
        "#,
    )
    .bind(organization_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM organization_members
        WHERE organization_id = $1 AND user_id = $2
        "#,
    )
    .bind(organization_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected();

    if deleted == 0 {
        return Err(Error::NotFound);
    }

    record_audit(
        &mut tx,
        organization_id,
        actor_id,
        action,
        json!({ "user_id": target_user_id }),
    )
    .await?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn list_teams_for_member(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
) -> Result<Vec<TeamView>> {
    let org_role =
        require_org_role(pool, user_id, organization_id, OrganizationRole::Guest).await?;

    let teams = sqlx::query_as::<_, Team>(
        r#"
        SELECT id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        FROM teams
        WHERE organization_id = $1
          AND deleted_at IS NULL
        ORDER BY is_default DESC, lower(name), id
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(teams.len());
    for team in teams {
        let membership: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT role FROM team_members
            WHERE team_id = $1 AND user_id = $2
            "#,
        )
        .bind(team.id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let role = if org_role.at_least(OrganizationRole::Admin) {
            Some(TeamRole::Maintainer)
        } else {
            match membership {
                Some((r,)) => Some(parse_team_role(&r)?),
                None if org_role == OrganizationRole::Guest => {
                    // Guests only see teams they belong to.
                    continue;
                }
                None => None,
            }
        };

        out.push(TeamView {
            id: team.id,
            organization_id: team.organization_id,
            name: team.name,
            is_default: team.is_default,
            archived_at: team.archived_at,
            deleted_at: team.deleted_at,
            role,
            created_at: team.created_at,
            updated_at: team.updated_at,
        });
    }
    Ok(out)
}

pub async fn create_team(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    req: CreateTeam,
) -> Result<TeamView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let name = normalize_name(&req.name)?;

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    let team = sqlx::query_as::<_, Team>(
        r#"
        INSERT INTO teams (organization_id, name, is_default)
        VALUES ($1, $2, FALSE)
        RETURNING id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "team.created",
        json!({ "team_id": team.id, "name": name }),
    )
    .await?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    Ok(TeamView {
        id: team.id,
        organization_id: team.organization_id,
        name: team.name,
        is_default: team.is_default,
        archived_at: team.archived_at,
        deleted_at: team.deleted_at,
        role: Some(TeamRole::Maintainer),
        created_at: team.created_at,
        updated_at: team.updated_at,
    })
}

/// Effective team role for `user_id` on `team_id`.
/// Org owners/admins act as Maintainer without a `team_members` row.
pub async fn effective_team_role(pool: &PgPool, user_id: Uuid, team_id: Uuid) -> Result<TeamRole> {
    let row: Option<(Uuid, Option<chrono::DateTime<chrono::Utc>>, String)> = sqlx::query_as(
        r#"
        SELECT t.organization_id, t.archived_at, m.role
        FROM teams t
        INNER JOIN organizations o ON o.id = t.organization_id
        INNER JOIN organization_members m ON m.organization_id = t.organization_id
          AND m.user_id = $2
        WHERE t.id = $1
          AND o.deleted_at IS NULL
          AND t.deleted_at IS NULL
        "#,
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let Some((organization_id, _archived_at, org_role_s)) = row else {
        return Err(Error::NotFound);
    };
    let org_role = parse_org_role(&org_role_s)?;
    if org_role.at_least(OrganizationRole::Admin) {
        return Ok(TeamRole::Maintainer);
    }

    let membership: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT role FROM team_members
        WHERE team_id = $1 AND user_id = $2
        "#,
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    match membership {
        Some((r,)) => parse_team_role(&r),
        None => {
            // Guests without a team seat, or members not on this team.
            let _ = organization_id;
            Err(Error::NotFound)
        }
    }
}

pub async fn require_team_role(
    pool: &PgPool,
    user_id: Uuid,
    team_id: Uuid,
    min: TeamRole,
) -> Result<TeamRole> {
    let role = effective_team_role(pool, user_id, team_id).await?;
    if !role.at_least(min) {
        return Err(Error::Forbidden);
    }
    Ok(role)
}

/// True when the team exists, org is live, and `archived_at` is set.
pub async fn team_is_archived(pool: &PgPool, team_id: Uuid) -> Result<bool> {
    let archived: Option<Option<chrono::DateTime<chrono::Utc>>> = sqlx::query_scalar(
        r#"
        SELECT t.archived_at
        FROM teams t
        INNER JOIN organizations o ON o.id = t.organization_id
        WHERE t.id = $1 AND o.deleted_at IS NULL AND t.deleted_at IS NULL
        "#,
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    match archived {
        Some(Some(_)) => Ok(true),
        Some(None) => Ok(false),
        None => Err(Error::NotFound),
    }
}

pub async fn require_team_writable(
    pool: &PgPool,
    user_id: Uuid,
    team_id: Uuid,
    min: TeamRole,
) -> Result<TeamRole> {
    if team_is_archived(pool, team_id).await? {
        return Err(Error::BadRequest("that team is archived".into()));
    }
    require_team_role(pool, user_id, team_id, min).await
}

/// Organization id for a live team (for audit events).
pub async fn organization_id_for_team(pool: &PgPool, team_id: Uuid) -> Result<Uuid> {
    sqlx::query_scalar(
        r#"
        SELECT t.organization_id
        FROM teams t
        INNER JOIN organizations o ON o.id = t.organization_id
        WHERE t.id = $1 AND o.deleted_at IS NULL AND t.deleted_at IS NULL
        "#,
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

/// Append an org audit event outside an existing transaction.
pub async fn append_organization_audit(
    pool: &PgPool,
    organization_id: Uuid,
    actor_user_id: Option<Uuid>,
    action: &str,
    payload: serde_json::Value,
) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    record_audit(&mut tx, organization_id, actor_user_id, action, payload).await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))
}

const ORG_DELETE_GRACE_DAYS: i64 = 30;
const AUDIT_RETENTION_DAYS: i64 = 365;

async fn load_team_in_org(pool: &PgPool, organization_id: Uuid, team_id: Uuid) -> Result<Team> {
    sqlx::query_as::<_, Team>(
        r#"
        SELECT id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        FROM teams
        WHERE id = $1 AND organization_id = $2
        "#,
    )
    .bind(team_id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

fn team_to_view(team: Team, role: Option<TeamRole>) -> TeamView {
    TeamView {
        id: team.id,
        organization_id: team.organization_id,
        name: team.name,
        is_default: team.is_default,
        archived_at: team.archived_at,
        deleted_at: team.deleted_at,
        role,
        created_at: team.created_at,
        updated_at: team.updated_at,
    }
}

/// Archive a team (read-only). Maintainers or org admins/owners.
pub async fn archive_team(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    team_id: Uuid,
) -> Result<TeamView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Guest).await?;
    require_team_role(pool, actor_id, team_id, TeamRole::Maintainer).await?;
    let team = load_team_in_org(pool, organization_id, team_id).await?;
    if team.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    if team.archived_at.is_some() {
        return Ok(team_to_view(team, Some(TeamRole::Maintainer)));
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let updated = sqlx::query_as::<_, Team>(
        r#"
        UPDATE teams
        SET archived_at = now(), updated_at = now()
        WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL AND archived_at IS NULL
        RETURNING id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        "#,
    )
    .bind(team_id)
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "team.archived",
        json!({ "team_id": team_id, "name": updated.name }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(team_to_view(updated, Some(TeamRole::Maintainer)))
}

/// Clear `archived_at` on a live team.
pub async fn restore_archived_team(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    team_id: Uuid,
) -> Result<TeamView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Guest).await?;
    require_team_role(pool, actor_id, team_id, TeamRole::Maintainer).await?;
    let team = load_team_in_org(pool, organization_id, team_id).await?;
    if team.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    if team.archived_at.is_none() {
        return Ok(team_to_view(team, Some(TeamRole::Maintainer)));
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let updated = sqlx::query_as::<_, Team>(
        r#"
        UPDATE teams
        SET archived_at = NULL, updated_at = now()
        WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
        RETURNING id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        "#,
    )
    .bind(team_id)
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "team.restored",
        json!({ "team_id": team_id, "name": updated.name }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(team_to_view(updated, Some(TeamRole::Maintainer)))
}

/// Soft-delete a non-default team and its live content (same `deleted_at` stamp for restore).
pub async fn soft_delete_team(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    team_id: Uuid,
) -> Result<()> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let team = load_team_in_org(pool, organization_id, team_id).await?;
    if team.is_default {
        return Err(Error::BadRequest(
            "the default team cannot be deleted".into(),
        ));
    }
    if team.deleted_at.is_some() {
        return Ok(());
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let stamp: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT now()")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        UPDATE teams
        SET deleted_at = $3, updated_at = $3
        WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(team_id)
    .bind(organization_id)
    .bind(stamp)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    for q in [
        "UPDATE categories SET deleted_at = $2, updated_at = $2 WHERE team_id = $1 AND deleted_at IS NULL",
        "UPDATE bookmarks SET deleted_at = $2, updated_at = $2 WHERE team_id = $1 AND deleted_at IS NULL",
        "UPDATE dashboards SET deleted_at = $2, updated_at = $2 WHERE team_id = $1 AND deleted_at IS NULL",
    ] {
        sqlx::query(q)
            .bind(team_id)
            .bind(stamp)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    }

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "team.deleted",
        json!({ "team_id": team_id, "name": team.name, "deleted_at": stamp }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Undo a team soft-delete within the grace window (restores content stamped with the team).
pub async fn restore_deleted_team(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    team_id: Uuid,
) -> Result<TeamView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let team = load_team_in_org(pool, organization_id, team_id).await?;
    let Some(stamp) = team.deleted_at else {
        return Ok(team_to_view(team, Some(TeamRole::Maintainer)));
    };

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let updated = sqlx::query_as::<_, Team>(
        r#"
        UPDATE teams
        SET deleted_at = NULL, updated_at = now()
        WHERE id = $1 AND organization_id = $2 AND deleted_at IS NOT NULL
        RETURNING id, organization_id, name, is_default, archived_at, deleted_at, created_at, updated_at
        "#,
    )
    .bind(team_id)
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    for q in [
        "UPDATE categories SET deleted_at = NULL, updated_at = now() WHERE team_id = $1 AND deleted_at = $2",
        "UPDATE bookmarks SET deleted_at = NULL, updated_at = now() WHERE team_id = $1 AND deleted_at = $2",
        "UPDATE dashboards SET deleted_at = NULL, updated_at = now() WHERE team_id = $1 AND deleted_at = $2",
    ] {
        sqlx::query(q)
            .bind(team_id)
            .bind(stamp)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    }

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "team.undeleted",
        json!({ "team_id": team_id, "name": updated.name }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(team_to_view(updated, Some(TeamRole::Maintainer)))
}

/// Owner-only soft delete. `confirm_name` must match the org name (trimmed).
/// Returns owner emails to notify (caller enqueues mail).
pub async fn soft_delete_organization(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    confirm_name: &str,
) -> Result<Vec<String>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Owner).await?;
    let org = sqlx::query_as::<_, Organization>(
        r#"
        SELECT id, name, created_at, updated_at, deleted_at, invite_links_enabled
        FROM organizations WHERE id = $1
        "#,
    )
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    if org.deleted_at.is_some() {
        return list_owner_emails(pool, organization_id).await;
    }
    if org.name != confirm_name.trim() {
        return Err(Error::BadRequest(
            "type the organization name to confirm deletion".into(),
        ));
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE organizations
        SET deleted_at = now(), updated_at = now()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(organization_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "organization.deleted",
        json!({ "name": org.name }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    list_owner_emails(pool, organization_id).await
}

pub async fn restore_organization(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
) -> Result<OrganizationWorkspace> {
    // membership_role hides soft-deleted orgs; check owner directly.
    let role: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT m.role
        FROM organization_members m
        WHERE m.organization_id = $1 AND m.user_id = $2
        "#,
    )
    .bind(organization_id)
    .bind(actor_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let Some((role_s,)) = role else {
        return Err(Error::NotFound);
    };
    if parse_org_role(&role_s)? != OrganizationRole::Owner {
        return Err(Error::Forbidden);
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let org = sqlx::query_as::<_, Organization>(
        r#"
        UPDATE organizations
        SET deleted_at = NULL, updated_at = now()
        WHERE id = $1 AND deleted_at IS NOT NULL
        RETURNING id, name, created_at, updated_at, deleted_at, invite_links_enabled
        "#,
    )
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::BadRequest(
        "organization is not pending deletion".into(),
    ))?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "organization.restored",
        json!({ "name": org.name }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    Ok(OrganizationWorkspace {
        id: org.id,
        name: org.name,
        role: OrganizationRole::Owner,
        deleted_at: None,
        invite_links_enabled: org.invite_links_enabled,
        created_at: org.created_at,
        updated_at: org.updated_at,
    })
}

pub async fn list_owner_emails(pool: &PgPool, organization_id: Uuid) -> Result<Vec<String>> {
    sqlx::query_scalar(
        r#"
        SELECT u.email
        FROM organization_members m
        INNER JOIN users u ON u.id = m.user_id
        WHERE m.organization_id = $1 AND m.role = 'owner'
        ORDER BY u.email
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn list_organization_audit_events(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    query: &linkdean_core::ListOrganizationAuditQuery,
) -> Result<Vec<linkdean_core::OrganizationAuditEventView>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0).max(0);

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Option<Uuid>,
            Option<String>,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT e.id, e.organization_id, e.actor_user_id, u.email, e.action, e.payload, e.created_at
        FROM organization_audit_events e
        LEFT JOIN users u ON u.id = e.actor_user_id
        WHERE e.organization_id = $1
          AND ($2::uuid IS NULL OR e.actor_user_id = $2)
          AND ($3::text IS NULL OR e.action = $3)
          AND ($4::timestamptz IS NULL OR e.created_at >= $4)
          AND ($5::timestamptz IS NULL OR e.created_at <= $5)
        ORDER BY e.created_at DESC, e.id DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(organization_id)
    .bind(query.actor_user_id)
    .bind(query.action.as_deref())
    .bind(query.from)
    .bind(query.to)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, organization_id, actor_user_id, actor_email, action, payload, created_at)| {
                linkdean_core::OrganizationAuditEventView {
                    id,
                    organization_id,
                    actor_user_id,
                    actor_email,
                    action,
                    payload,
                    created_at,
                }
            },
        )
        .collect())
}

/// Hard-delete orgs past the grace window. Cascades teams/content via FKs.
pub async fn purge_organizations_past_grace(pool: &PgPool) -> Result<u64> {
    let res = sqlx::query(
        r#"
        DELETE FROM organizations
        WHERE deleted_at IS NOT NULL
          AND deleted_at < now() - ($1::text || ' days')::interval
        "#,
    )
    .bind(ORG_DELETE_GRACE_DAYS.to_string())
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(res.rows_affected())
}

/// Hard-delete soft-deleted teams past grace (cascades team_id content).
pub async fn purge_teams_past_grace(pool: &PgPool) -> Result<u64> {
    let res = sqlx::query(
        r#"
        DELETE FROM teams
        WHERE deleted_at IS NOT NULL
          AND deleted_at < now() - ($1::text || ' days')::interval
        "#,
    )
    .bind(ORG_DELETE_GRACE_DAYS.to_string())
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(res.rows_affected())
}

pub async fn prune_organization_audit_events(pool: &PgPool) -> Result<u64> {
    let res = sqlx::query(
        r#"
        DELETE FROM organization_audit_events
        WHERE created_at < now() - ($1::text || ' days')::interval
        "#,
    )
    .bind(AUDIT_RETENTION_DAYS.to_string())
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(res.rows_affected())
}
