//! Restricted folders in team spaces (Orgs Phase A4). Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    CreateCategory, CreateOrganization, Error, OrganizationRole, SetCategoryRestriction, TeamRole,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping restricted_folders tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for restricted_folders tests");
    db::migrate(&pool)
        .await
        .expect("migrate for restricted_folders tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@restricted-folders.test", Uuid::new_v4())
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

async fn team_folder(pool: &sqlx::PgPool, actor: &linkdean_core::User, team_id: Uuid) -> Uuid {
    db::create_category(
        pool,
        actor.id,
        CreateCategory {
            name: format!("Folder {}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect("folder")
    .id
}

#[tokio::test]
async fn editor_without_allowlist_cannot_see_restricted_folder() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let editor = user(&pool, "ed").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, editor.id, OrganizationRole::Member)
        .await
        .expect("member");

    let folder = team_folder(&pool, &owner, team_id).await;
    db::set_category_restriction(
        &pool,
        owner.id,
        folder,
        SetCategoryRestriction { member_ids: vec![] },
    )
    .await
    .expect("restrict");

    let listed = db::list_team_categories(&pool, editor.id, team_id, true, false)
        .await
        .expect("list");
    assert!(!listed.iter().any(|c| c.id == folder));

    assert!(matches!(
        db::require_category_read(&pool, editor.id, folder).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn allowlisted_editor_keeps_write_access() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let editor = user(&pool, "ed").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, editor.id, OrganizationRole::Member)
        .await
        .expect("member");

    let folder = team_folder(&pool, &owner, team_id).await;
    db::set_category_restriction(
        &pool,
        owner.id,
        folder,
        SetCategoryRestriction {
            member_ids: vec![editor.id],
        },
    )
    .await
    .expect("restrict");

    let listed = db::list_team_categories(&pool, editor.id, team_id, true, false)
        .await
        .expect("list");
    assert!(listed.iter().any(|c| c.id == folder));

    let (_, access) = db::require_category_write(&pool, editor.id, folder)
        .await
        .expect("write");
    assert!(access.can_write());
    assert!(!access.is_owner()); // Editor maps to Write, not Owner
}

#[tokio::test]
async fn org_admin_sees_restricted_without_allowlist() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let admin = user(&pool, "adm").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, admin.id, OrganizationRole::Admin)
        .await
        .expect("admin");

    let folder = team_folder(&pool, &owner, team_id).await;
    db::set_category_restriction(
        &pool,
        owner.id,
        folder,
        SetCategoryRestriction { member_ids: vec![] },
    )
    .await
    .expect("restrict");

    let listed = db::list_team_categories(&pool, admin.id, team_id, true, false)
        .await
        .expect("list");
    assert!(listed.iter().any(|c| c.id == folder && c.is_restricted));

    let role = db::effective_team_role(&pool, admin.id, team_id)
        .await
        .expect("role");
    assert_eq!(role, TeamRole::Maintainer);
}

#[tokio::test]
async fn restriction_inherits_to_child_folder() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let editor = user(&pool, "ed").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, editor.id, OrganizationRole::Member)
        .await
        .expect("member");

    let parent = team_folder(&pool, &owner, team_id).await;
    let child = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Child".into(),
            parent_id: Some(parent),
            is_hidden: false,
            vault_id: None,
            icon: None,
            team_id: Some(team_id),
        },
    )
    .await
    .expect("child")
    .id;

    db::set_category_restriction(
        &pool,
        owner.id,
        parent,
        SetCategoryRestriction { member_ids: vec![] },
    )
    .await
    .expect("restrict parent");

    assert!(matches!(
        db::require_category_read(&pool, editor.id, child).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn editor_cannot_set_restriction() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let editor = user(&pool, "ed").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, editor.id, OrganizationRole::Member)
        .await
        .expect("member");
    let folder = team_folder(&pool, &owner, team_id).await;

    let err = db::set_category_restriction(
        &pool,
        editor.id,
        folder,
        SetCategoryRestriction {
            member_ids: vec![editor.id],
        },
    )
    .await
    .expect_err("editor set");
    assert!(matches!(err, Error::Forbidden));
}

#[tokio::test]
async fn clear_restriction_restores_team_visibility() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let editor = user(&pool, "ed").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, editor.id, OrganizationRole::Member)
        .await
        .expect("member");
    let folder = team_folder(&pool, &owner, team_id).await;
    db::set_category_restriction(
        &pool,
        owner.id,
        folder,
        SetCategoryRestriction { member_ids: vec![] },
    )
    .await
    .expect("restrict");
    db::clear_category_restriction(&pool, owner.id, folder)
        .await
        .expect("clear");

    let listed = db::list_team_categories(&pool, editor.id, team_id, true, false)
        .await
        .expect("list");
    assert!(listed.iter().any(|c| c.id == folder && !c.is_restricted));
}

#[tokio::test]
async fn personal_folder_cannot_be_restricted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
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
    .expect("personal")
    .id;

    let err = db::set_category_restriction(
        &pool,
        owner.id,
        folder,
        SetCategoryRestriction { member_ids: vec![] },
    )
    .await
    .expect_err("personal restrict");
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn restricted_folder_hides_filed_bookmark_from_editor() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own").await;
    let editor = user(&pool, "ed").await;
    let (org_id, team_id) = org_with_general(&pool, &owner).await;
    db::add_organization_member(&pool, org_id, editor.id, OrganizationRole::Member)
        .await
        .expect("member");

    let folder = team_folder(&pool, &owner, team_id).await;
    let bm = db::create_bookmark(
        &pool,
        owner.id,
        linkdean_core::CreateBookmark {
            url: format!("https://restricted.example/{}", Uuid::new_v4()),
            title: Some("Secret".into()),
            category_id: Some(folder),
            category_ids: vec![folder],
            team_id: Some(team_id),
            ..Default::default()
        },
        &linkdean_core::VaultKeyring::default(),
    )
    .await
    .expect("bookmark");

    db::set_category_restriction(
        &pool,
        owner.id,
        folder,
        SetCategoryRestriction { member_ids: vec![] },
    )
    .await
    .expect("restrict");

    assert!(!db::bookmark_can_read(&pool, editor.id, bm.id)
        .await
        .expect("read check"));
    assert!(db::bookmark_can_read(&pool, owner.id, bm.id)
        .await
        .expect("owner read"));
}
