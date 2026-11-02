//! Duplicate detection only counts live bookmarks.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
use linkdean_core::{CreateBookmark, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping duplicate tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("dup-{}@duplicates.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn bookmark(pool: &PgPool, owner: &User, url: &str) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: url.to_string(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn trash(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE bookmarks SET deleted_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .expect("trash");
}

#[tokio::test]
async fn a_trashed_copy_is_not_a_duplicate() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let url = format!("https://dup.example/{}", Uuid::new_v4());
    let _live = bookmark(&pool, &owner, &url).await;
    let trashed = bookmark(&pool, &owner, &url).await;
    trash(&pool, trashed).await;

    assert!(db::find_duplicate_urls(&pool, owner.id)
        .await
        .expect("urls")
        .is_empty());
    assert!(db::find_duplicate_groups(&pool, owner.id)
        .await
        .expect("groups")
        .is_empty());
}

#[tokio::test]
async fn groups_leave_out_trashed_copies() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let url = format!("https://dup.example/{}", Uuid::new_v4());
    let a = bookmark(&pool, &owner, &url).await;
    let b = bookmark(&pool, &owner, &url).await;
    let trashed = bookmark(&pool, &owner, &url).await;
    trash(&pool, trashed).await;

    let groups = db::find_duplicate_groups(&pool, owner.id)
        .await
        .expect("groups");
    assert_eq!(groups.len(), 1);
    let mut ids: Vec<Uuid> = groups[0].1.iter().map(|v| v.id).collect();
    ids.sort();
    let mut want = vec![a, b];
    want.sort();
    assert_eq!(ids, want);
}
