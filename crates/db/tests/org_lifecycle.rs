//! Org / team lifecycle (archive, soft-delete, audit). Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    CreateOrganization, CreateTeam, Error, ListOrganizationAuditQuery, OrganizationRole,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping org lifecycle tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for org lifecycle tests");
    db::migrate(&pool)
        .await
        .expect("migrate for org lifecycle tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@org-life.test", Uuid::new_v4())
}

async fn user(pool: &sqlx::PgPool, prefix: &str) -> linkdean_core::User {
    db::create_user(
        pool,
        &unique_email(prefix),
        Some("password-long-enough"),
        Some(prefix),
        None,
        None,
    )
    .await
    .expect("user")
}

#[tokio::test]
async fn archive_blocks_writes_and_restore_clears() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "arch-owner").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Archive Co".into(),
        },
    )
    .await
    .expect("org");
    let team = db::create_team(
        &pool,
        owner.id,
        org.id,
        CreateTeam {
            name: "Squad".into(),
        },
    )
    .await
    .expect("team");

    let archived = db::archive_team(&pool, owner.id, org.id, team.id)
        .await
        .expect("archive");
    assert!(archived.archived_at.is_some());
    assert!(matches!(
        db::require_team_writable(&pool, owner.id, team.id, linkdean_core::TeamRole::Editor).await,
        Err(Error::BadRequest(_))
    ));

    let restored = db::restore_archived_team(&pool, owner.id, org.id, team.id)
        .await
        .expect("unarchive");
    assert!(restored.archived_at.is_none());
    db::require_team_writable(&pool, owner.id, team.id, linkdean_core::TeamRole::Editor)
        .await
        .expect("writable again");
}

#[tokio::test]
async fn soft_delete_team_hides_from_list_and_restores() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "del-owner").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Delete Co".into(),
        },
    )
    .await
    .expect("org");
    let team = db::create_team(
        &pool,
        owner.id,
        org.id,
        CreateTeam {
            name: "Temp".into(),
        },
    )
    .await
    .expect("team");

    let default = db::list_teams_for_member(&pool, owner.id, org.id)
        .await
        .expect("teams")
        .into_iter()
        .find(|t| t.is_default)
        .expect("default");
    assert!(matches!(
        db::soft_delete_team(&pool, owner.id, org.id, default.id).await,
        Err(Error::BadRequest(_))
    ));

    db::soft_delete_team(&pool, owner.id, org.id, team.id)
        .await
        .expect("soft delete");
    let listed = db::list_teams_for_member(&pool, owner.id, org.id)
        .await
        .expect("list");
    assert!(!listed.iter().any(|t| t.id == team.id));

    let restored = db::restore_deleted_team(&pool, owner.id, org.id, team.id)
        .await
        .expect("restore");
    assert!(restored.deleted_at.is_none());
    assert!(db::list_teams_for_member(&pool, owner.id, org.id)
        .await
        .expect("list2")
        .iter()
        .any(|t| t.id == team.id));
}

#[tokio::test]
async fn org_soft_delete_grace_and_audit() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "org-del-owner").await;
    let member = user(&pool, "org-del-member").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Grace Org".into(),
        },
    )
    .await
    .expect("org");
    db::add_organization_member(&pool, org.id, member.id, OrganizationRole::Member)
        .await
        .expect("add member");

    assert!(matches!(
        db::soft_delete_organization(&pool, owner.id, org.id, "Wrong").await,
        Err(Error::BadRequest(_))
    ));
    assert!(matches!(
        db::soft_delete_organization(&pool, member.id, org.id, "Grace Org").await,
        Err(Error::Forbidden)
    ));

    db::soft_delete_organization(&pool, owner.id, org.id, "Grace Org")
        .await
        .expect("delete");

    let owner_list = db::list_organizations_for_user(&pool, owner.id)
        .await
        .expect("owner list");
    assert!(owner_list
        .iter()
        .any(|o| o.id == org.id && o.deleted_at.is_some()));
    assert!(db::list_organizations_for_user(&pool, member.id)
        .await
        .expect("member list")
        .iter()
        .all(|o| o.id != org.id));

    db::restore_organization(&pool, owner.id, org.id)
        .await
        .expect("restore");
    assert!(db::list_organizations_for_user(&pool, member.id)
        .await
        .expect("member after")
        .iter()
        .any(|o| o.id == org.id && o.deleted_at.is_none()));

    let audit = db::list_organization_audit_events(
        &pool,
        owner.id,
        org.id,
        &ListOrganizationAuditQuery {
            action: Some("organization.deleted".into()),
            limit: Some(10),
            ..Default::default()
        },
    )
    .await
    .expect("audit");
    assert!(!audit.is_empty());
}
