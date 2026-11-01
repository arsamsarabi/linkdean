//! Who can see a profile and its bookmarks.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping profile_visibility tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    db::create_user(
        pool,
        &format!("{prefix}-{}@visibility.test", Uuid::new_v4()),
        Some("password-long"),
        None,
        None,
        None,
    )
    .await
    .expect("user")
}

async fn category(pool: &PgPool, owner: &User) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Visibility-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn bookmark(pool: &PgPool, owner: &User, category_ids: Vec<Uuid>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://visibility.example/{}", Uuid::new_v4()),
            title: Some("Bookmark".into()),
            category_ids,
            ..Default::default()
        },
        None,
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn new_rows_default_to_invisible() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let bm = bookmark(&pool, &owner, vec![cat]).await;

    assert_eq!(owner.profile_visibility, "hidden");
    let (visibility,): (String,) =
        sqlx::query_as("SELECT visibility FROM categories WHERE id = $1")
            .bind(cat)
            .fetch_one(&pool)
            .await
            .expect("category row");
    assert_eq!(visibility, "private");
    let (is_private,): (bool,) = sqlx::query_as("SELECT is_private FROM bookmarks WHERE id = $1")
        .bind(bm)
        .fetch_one(&pool)
        .await
        .expect("bookmark row");
    assert!(!is_private);
}

#[tokio::test]
async fn vault_category_cannot_leave_private_at_db_level() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "vault").await;
    let cat = category(&pool, &owner).await;
    sqlx::query("UPDATE categories SET is_vault = TRUE WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await
        .expect("mark vault");
    let denied = sqlx::query("UPDATE categories SET visibility = 'members' WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await;
    assert!(denied.is_err());
}
