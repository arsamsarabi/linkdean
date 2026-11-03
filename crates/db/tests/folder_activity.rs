//! The folder list reports when each folder's bookmarks last changed.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping folder activity tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("folder-activity-{}@limits.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn folder(pool: &PgPool, owner: &User, name: &str) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: name.into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder")
    .id
}

async fn bookmark(pool: &PgPool, owner: &User, folder: Uuid) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            vault_id: None,
            url: format!("https://example.com/{}", Uuid::new_v4()),
            title: Some("Activity".into()),
            description: None,
            notes: None,
            category_id: Some(folder),
            category_ids: vec![],
            tags: vec![],
            is_favorite: false,
            is_unread: false,
            is_pinned: false,
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn folders_report_their_latest_live_bookmark() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let older = folder(&pool, &owner, "Older").await;
    let newer = folder(&pool, &owner, "Newer").await;
    let empty = folder(&pool, &owner, "Empty").await;
    bookmark(&pool, &owner, older).await;
    let newest = bookmark(&pool, &owner, newer).await;

    let cats = db::list_categories(&pool, owner.id, false, &[], false)
        .await
        .expect("list");
    let at = |id: Uuid| cats.iter().find(|c| c.id == id).unwrap().last_bookmark_at;
    assert!(at(newer).expect("newer") > at(older).expect("older"));
    assert_eq!(at(empty), None);

    db::delete_bookmark(&pool, owner.id, newest)
        .await
        .expect("delete");
    let cats = db::list_categories(&pool, owner.id, false, &[], false)
        .await
        .expect("list");
    let after = cats.iter().find(|c| c.id == newer).unwrap();
    assert_eq!(
        after.last_bookmark_at, None,
        "deleted bookmarks do not count"
    );
}
