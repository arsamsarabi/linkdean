//! Organization membership. Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    CreateOrganization, CreateTeam, Error, OrganizationRole, PatchOrganizationMember,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping organizations tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for organizations tests");
    db::migrate(&pool)
        .await
        .expect("migrate for organizations tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@orgs.test", Uuid::new_v4())
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
async fn create_lists_default_team_and_isolates_members() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "owner").await;
    let b = user(&pool, "outsider").await;

    let org = db::create_organization(
        &pool,
        &a,
        CreateOrganization {
            name: "  Acme Inc  ".into(),
        },
    )
    .await
    .expect("create");
    assert_eq!(org.name, "Acme Inc");
    assert_eq!(org.role, OrganizationRole::Owner);

    let listed_a = db::list_organizations_for_user(&pool, a.id)
        .await
        .expect("list a");
    assert!(listed_a.iter().any(|o| o.id == org.id));
    assert!(db::list_organizations_for_user(&pool, b.id)
        .await
        .expect("list b")
        .is_empty());

    let teams = db::list_teams_for_member(&pool, a.id, org.id)
        .await
        .expect("teams");
    assert_eq!(teams.len(), 1);
    assert!(teams[0].is_default);
    assert_eq!(teams[0].name, "General");
    assert_eq!(teams[0].role, Some(linkdean_core::TeamRole::Maintainer));

    assert!(matches!(
        db::get_organization_for_member(&pool, b.id, org.id).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn last_owner_cannot_leave_until_another_is_promoted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "solo").await;
    let b = user(&pool, "peer").await;
    let org = db::create_organization(
        &pool,
        &a,
        CreateOrganization {
            name: "Solo Org".into(),
        },
    )
    .await
    .expect("create");

    let err = db::leave_organization(&pool, a.id, org.id)
        .await
        .expect_err("last owner leave");
    assert!(matches!(err, Error::BadRequest(_)));

    db::add_organization_member(&pool, org.id, b.id, OrganizationRole::Admin)
        .await
        .expect("add admin");
    db::patch_organization_member(&pool, a.id, org.id, b.id, OrganizationRole::Owner)
        .await
        .expect("promote");

    db::leave_organization(&pool, a.id, org.id)
        .await
        .expect("leave after promote");
    assert!(db::list_organizations_for_user(&pool, a.id)
        .await
        .expect("list")
        .is_empty());
}

#[tokio::test]
async fn guest_cannot_see_member_directory_and_only_sees_assigned_teams() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "host").await;
    let g = user(&pool, "guest").await;
    let org = db::create_organization(
        &pool,
        &a,
        CreateOrganization {
            name: "Guest Org".into(),
        },
    )
    .await
    .expect("create");
    db::add_organization_member(&pool, org.id, g.id, OrganizationRole::Guest)
        .await
        .expect("add guest");

    assert!(matches!(
        db::list_organization_members(&pool, g.id, org.id).await,
        Err(Error::Forbidden)
    ));

    let guest_teams = db::list_teams_for_member(&pool, g.id, org.id)
        .await
        .expect("guest teams");
    assert!(guest_teams.is_empty(), "guest not on default team");

    let extra = db::create_team(
        &pool,
        a.id,
        org.id,
        CreateTeam {
            name: "Contractors".into(),
        },
    )
    .await
    .expect("team");
    // Attach guest to the extra team via SQL for this membership-only slice.
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'viewer')")
        .bind(extra.id)
        .bind(g.id)
        .execute(&pool)
        .await
        .expect("team member");

    let guest_teams = db::list_teams_for_member(&pool, g.id, org.id)
        .await
        .expect("guest teams after");
    assert_eq!(guest_teams.len(), 1);
    assert_eq!(guest_teams[0].id, extra.id);
}

#[tokio::test]
async fn create_blocked_when_instance_closes_org_creation() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "locked").await;
    db::set_organizations_create_open(&pool, false)
        .await
        .expect("close");
    let err = db::create_organization(
        &pool,
        &a,
        CreateOrganization {
            name: "Nope".into(),
        },
    )
    .await
    .expect_err("forbidden");
    assert!(matches!(err, Error::Forbidden));
    db::set_organizations_create_open(&pool, true)
        .await
        .expect("reopen");
}

#[tokio::test]
async fn patch_member_round_trip_via_view() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "boss").await;
    let b = user(&pool, "staff").await;
    let org = db::create_organization(
        &pool,
        &a,
        CreateOrganization {
            name: "Roles".into(),
        },
    )
    .await
    .expect("create");
    db::add_organization_member(&pool, org.id, b.id, OrganizationRole::Member)
        .await
        .expect("add");
    let patched = db::patch_organization_member(
        &pool,
        a.id,
        org.id,
        b.id,
        PatchOrganizationMember {
            role: OrganizationRole::Admin,
        }
        .role,
    )
    .await
    .expect("patch");
    assert_eq!(patched.role, OrganizationRole::Admin);
}
