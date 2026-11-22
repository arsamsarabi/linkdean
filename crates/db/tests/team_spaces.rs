//! Team-space content ownership (Orgs Phase A3). Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateOrganization, Error, MoveToTeam, TeamRole, VaultKeyring,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping team_spaces tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for team_spaces tests");
    db::migrate(&pool)
        .await
        .expect("migrate for team_spaces tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@team-spaces.test", Uuid::new_v4())
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

async fn org_with_general(pool: &sqlx::PgPool, owner: &linkdean_core::User) -> (Uuid, Uuid) {
    let ws = db::create_organization(
        pool,
        owner,
        CreateOrganization {
            name: format!("Org {}", Uuid::new_v4()),
        },
    )
    .await
    .expect("org");
    let teams = db::list_teams_for_member(pool, owner.id, ws.id)
        .await
        .expect("teams");
    let general = teams.into_iter().find(|t| t.is_default).expect("General");
    (ws.id, general.id)
}

#[tokio::test]
async fn xor_and_team_isolation() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let outsider = user(&pool, "out").await;
    let (_, team_id) = org_with_general(&pool, &owner).await;

    let folder = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Team folder".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect("team folder");
    assert!(folder.owner_user_id.is_none());
    assert_eq!(folder.team_id, Some(team_id));

    let personal = db::list_categories(&pool, owner.id, true, &[], false)
        .await
        .expect("personal list");
    assert!(!personal.iter().any(|c| c.id == folder.id));

    let team_list = db::list_team_categories(&pool, owner.id, team_id, true, false)
        .await
        .expect("team list");
    assert!(team_list.iter().any(|c| c.id == folder.id));

    assert!(matches!(
        db::list_team_categories(&pool, outsider.id, team_id, true, false).await,
        Err(Error::NotFound | Error::Forbidden)
    ));
}

#[tokio::test]
async fn org_admin_without_team_row_can_write() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let admin = user(&pool, "adm").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(
        &pool,
        org_id,
        admin.id,
        linkdean_core::OrganizationRole::Admin,
    )
    .await
    .expect("add admin");
    // Admin is not on team_members for a non-default team; for General they may not be either
    // when added as admin after create — effective role is still Maintainer via org role.
    let role = db::effective_team_role(&pool, admin.id, team_id)
        .await
        .expect("role");
    assert_eq!(role, TeamRole::Maintainer);

    let folder = db::create_category(
        &pool,
        admin.id,
        CreateCategory {
            name: "Admin folder".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect("admin create");
    assert_eq!(folder.team_id, Some(team_id));
}

#[tokio::test]
async fn viewer_cannot_create() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let viewer = user(&pool, "view").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(
        &pool,
        org_id,
        viewer.id,
        linkdean_core::OrganizationRole::Member,
    )
    .await
    .expect("member");
    // Force Viewer on General.
    sqlx::query("UPDATE team_members SET role = 'viewer' WHERE team_id = $1 AND user_id = $2")
        .bind(team_id)
        .bind(viewer.id)
        .execute(&pool)
        .await
        .expect("viewer role");

    let err = db::create_category(
        &pool,
        viewer.id,
        CreateCategory {
            name: "Nope".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect_err("viewer create");
    assert!(matches!(err, Error::Forbidden));

    let _ = db::list_team_categories(&pool, viewer.id, team_id, true, false)
        .await
        .expect("viewer can list");
}

#[tokio::test]
async fn move_requires_ack_then_transfers() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let (_, team_id) = org_with_general(&pool, &owner).await;
    let folder = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Personal".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: None,
        },
    )
    .await
    .expect("personal folder");

    let err = db::move_category_to_team(
        &pool,
        owner.id,
        folder.id,
        MoveToTeam {
            team_id,
            acknowledge_org_ownership: false,
        },
    )
    .await
    .expect_err("no ack");
    assert!(matches!(err, Error::BadRequest(_)));

    let moved = db::move_category_to_team(
        &pool,
        owner.id,
        folder.id,
        MoveToTeam {
            team_id,
            acknowledge_org_ownership: true,
        },
    )
    .await
    .expect("move");
    assert!(moved.owner_user_id.is_none());
    assert_eq!(moved.team_id, Some(team_id));
}

#[tokio::test]
async fn copy_to_personal_leaves_source() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let (_, team_id) = org_with_general(&pool, &owner).await;
    let folder = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Shared notes".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect("team folder");

    let copy = db::copy_category_to_personal(&pool, owner.id, folder.id)
        .await
        .expect("copy");
    assert_eq!(copy.owner_user_id, Some(owner.id));
    assert!(copy.team_id.is_none());
    assert_ne!(copy.id, folder.id);

    let still = db::get_category_by_id(&pool, folder.id).await.expect("src");
    assert_eq!(still.team_id, Some(team_id));
}

#[tokio::test]
async fn vault_refused_in_team_space() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let (_, team_id) = org_with_general(&pool, &owner).await;
    let err = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Vaulty".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: Some(Uuid::new_v4()),
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect_err("vault+team");
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn tags_unique_per_team() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let (_, team_a) = org_with_general(&pool, &owner).await;
    let (_, team_b) = org_with_general(&pool, &owner).await;

    let a = db::ensure_tags_in_space(&pool, None, Some(team_a), &["alpha".into()])
        .await
        .expect("tags a");
    let b = db::ensure_tags_in_space(&pool, None, Some(team_b), &["alpha".into()])
        .await
        .expect("tags b");
    assert_ne!(a[0].id, b[0].id);

    let again = db::ensure_tags_in_space(&pool, None, Some(team_a), &["alpha".into()])
        .await
        .expect("idempotent");
    assert_eq!(again[0].id, a[0].id);
}

#[tokio::test]
async fn team_bookmark_create() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let (_, team_id) = org_with_general(&pool, &owner).await;
    let folder = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Links".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect("folder");
    let bm = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://example.test/team".into(),
            title: Some("Team link".into()),
            category_id: Some(folder.id),
            team_id: Some(team_id),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark");
    assert!(bm.owner_user_id.is_none());
    assert_eq!(bm.team_id, Some(team_id));
}
