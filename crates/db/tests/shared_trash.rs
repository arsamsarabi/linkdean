//! Trash belongs to the owner: invitees lose trashed folders and bookmarks.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, Error, ShareAccess, UpdateBookmark, User,
    VaultKeyring,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping shared trash tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@trash.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn folder(pool: &PgPool, owner: &User, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: "Folder".into(),
            parent_id,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder")
    .id
}

async fn share(pool: &PgPool, owner: &User, folder: Uuid, invitee: &User) {
    db::create_share(
        pool,
        owner.id,
        folder,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("share");
    for invite in db::list_share_invites(pool, invitee.id)
        .await
        .expect("inbox")
    {
        db::accept_share_invite(pool, invitee.id, invite.id)
            .await
            .expect("accept");
    }
}

async fn bookmark(pool: &PgPool, owner: &User, folder: Uuid) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://trash.example/{}", Uuid::new_v4()),
            title: Some("Shared".into()),
            category_id: Some(folder),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn listed(pool: &PgPool, viewer: &User, folder: Option<Uuid>) -> Result<Vec<Uuid>, Error> {
    Ok(db::list_bookmarks(
        pool,
        viewer.id,
        folder,
        false,
        &VaultKeyring::default(),
        true,
    )
    .await?
    .into_iter()
    .map(|b| b.id)
    .collect())
}

async fn shared_folders(pool: &PgPool, invitee: &User) -> Vec<Uuid> {
    db::list_shared_categories(pool, invitee.id, true)
        .await
        .expect("shared")
        .into_iter()
        .map(|(c, _)| c.id)
        .collect()
}

#[tokio::test]
async fn invitees_lose_a_trashed_bookmark() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let f = folder(&pool, &owner, None).await;
    share(&pool, &owner, f, &invitee).await;
    let b = bookmark(&pool, &owner, f).await;
    db::delete_bookmark(&pool, owner.id, b)
        .await
        .expect("trash");

    assert!(matches!(
        db::get_bookmark_raw(&pool, invitee.id, b).await,
        Err(Error::NotFound)
    ));
    assert!(!listed(&pool, &invitee, Some(f))
        .await
        .expect("list")
        .contains(&b));
    let edit = db::update_bookmark(
        &pool,
        invitee.id,
        b,
        UpdateBookmark {
            title: Some("Changed".into()),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(edit.is_err(), "{edit:?}");
    // The owner still sees it in their trash.
    assert!(listed(&pool, &owner, Some(f))
        .await
        .expect("owner")
        .contains(&b));
}

#[tokio::test]
async fn invitees_lose_a_trashed_folder() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let f = folder(&pool, &owner, None).await;
    let child = folder(&pool, &owner, Some(f)).await;
    share(&pool, &owner, f, &invitee).await;
    db::delete_category(&pool, owner.id, f)
        .await
        .expect("trash");

    for id in [f, child] {
        assert!(matches!(
            db::category_access(&pool, invitee.id, id).await,
            Err(Error::NotFound)
        ));
        assert!(listed(&pool, &invitee, Some(id)).await.is_err());
    }
    assert!(shared_folders(&pool, &invitee).await.is_empty());
}

#[tokio::test]
async fn a_live_share_under_a_trashed_parent_is_hidden() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let parent = folder(&pool, &owner, None).await;
    let shared = folder(&pool, &owner, Some(parent)).await;
    share(&pool, &owner, shared, &invitee).await;
    let b = bookmark(&pool, &owner, shared).await;
    // Only the parent is trashed, as after restoring just the child.
    sqlx::query("UPDATE categories SET deleted_at = now() WHERE id = $1")
        .bind(parent)
        .execute(&pool)
        .await
        .expect("trash parent");

    assert!(matches!(
        db::category_access(&pool, invitee.id, shared).await,
        Err(Error::NotFound)
    ));
    assert!(shared_folders(&pool, &invitee).await.is_empty());
    assert!(matches!(
        db::get_bookmark_raw(&pool, invitee.id, b).await,
        Err(Error::NotFound)
    ));
}
