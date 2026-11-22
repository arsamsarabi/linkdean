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

async fn record_audit(
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
        RETURNING id, name, created_at, updated_at, deleted_at
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
        RETURNING id, organization_id, name, is_default, archived_at, created_at, updated_at
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
        created_at: org.created_at,
        updated_at: org.updated_at,
    })
}

pub async fn list_organizations_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<OrganizationWorkspace>> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT o.id, o.name, m.role, o.created_at, o.updated_at
        FROM organizations o
        INNER JOIN organization_members m ON m.organization_id = o.id
        WHERE m.user_id = $1
          AND o.deleted_at IS NULL
        ORDER BY lower(o.name), o.id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter()
        .map(|(id, name, role, created_at, updated_at)| {
            Ok(OrganizationWorkspace {
                id,
                name,
                role: parse_org_role(&role)?,
                created_at,
                updated_at,
            })
        })
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
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT o.id, o.name, m.role, o.created_at, o.updated_at
        FROM organizations o
        INNER JOIN organization_members m ON m.organization_id = o.id
        WHERE o.id = $1
          AND m.user_id = $2
          AND o.deleted_at IS NULL
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
        created_at: row.3,
        updated_at: row.4,
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
        SELECT id, organization_id, name, is_default, archived_at, created_at, updated_at
        FROM teams
        WHERE organization_id = $1
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
        RETURNING id, organization_id, name, is_default, archived_at, created_at, updated_at
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
        role: Some(TeamRole::Maintainer),
        created_at: team.created_at,
        updated_at: team.updated_at,
    })
}
