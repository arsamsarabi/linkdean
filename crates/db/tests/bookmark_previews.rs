//! Which bookmarks an import queues for enrichment.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping bookmark preview tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@previews.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn bookmark(pool: &PgPool, owner: &User) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://previews.example/{}", Uuid::new_v4()),
            ..Default::default()
        },
        None,
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn lists_only_this_users_active_bookmarks_without_previews() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let other = user(&pool, "other").await;

    let waiting = bookmark(&pool, &owner).await;
    let captured = bookmark(&pool, &owner).await;
    let trashed = bookmark(&pool, &owner).await;
    bookmark(&pool, &other).await;

    db::set_bookmark_has_preview(&pool, captured, true)
        .await
        .expect("mark preview");
    sqlx::query("UPDATE bookmarks SET deleted_at = now() WHERE id = $1")
        .bind(trashed)
        .execute(&pool)
        .await
        .expect("trash");

    let rows = db::list_user_bookmarks_needing_preview(&pool, owner.id)
        .await
        .expect("list");
    assert_eq!(rows, vec![(owner.id, waiting)]);
}
