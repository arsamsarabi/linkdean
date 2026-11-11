//! Folder and tag suggestions for filing an Unsorted bookmark.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping filing suggestion tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool) -> User {
    let email = format!("file-{}@filing.test", Uuid::new_v4());
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

async fn bookmark(
    pool: &PgPool,
    owner: &User,
    url: &str,
    folders: Vec<Uuid>,
    tags: &[&str],
) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: url.into(),
            category_ids: folders,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn suggests_folders_and_tags_used_on_the_same_site_most_used_first() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let rust = folder(&pool, &me, "Rust").await;
    let reading = folder(&pool, &me, "Reading").await;
    let other_site = folder(&pool, &me, "Elsewhere").await;
    let host = format!("{}.example", Uuid::new_v4().simple());

    bookmark(
        &pool,
        &me,
        &format!("https://{host}/a"),
        vec![rust],
        &["lang", "systems"],
    )
    .await;
    bookmark(
        &pool,
        &me,
        &format!("https://www.{host}/b"),
        vec![rust, reading],
        &["lang"],
    )
    .await;
    bookmark(
        &pool,
        &me,
        "https://elsewhere.example/c",
        vec![other_site],
        &["nope"],
    )
    .await;
    let trashed = bookmark(
        &pool,
        &me,
        &format!("https://{host}/d"),
        vec![other_site],
        &["gone"],
    )
    .await;
    sqlx::query("UPDATE bookmarks SET deleted_at = now() WHERE id = $1")
        .bind(trashed)
        .execute(&pool)
        .await
        .expect("trash");
    // Another person's bookmark on the same site is never used.
    let them = person(&pool).await;
    let theirs = folder(&pool, &them, "Theirs").await;
    bookmark(
        &pool,
        &them,
        &format!("https://{host}/e"),
        vec![theirs],
        &["private"],
    )
    .await;

    let unsorted = bookmark(
        &pool,
        &me,
        &format!("https://{}/new?x=1", host.to_uppercase()),
        vec![],
        &[],
    )
    .await;
    let s = db::filing_suggestions(&pool, me.id, unsorted)
        .await
        .expect("suggest");

    let folders: Vec<(Uuid, i64)> = s.folders.iter().map(|f| (f.id, f.count)).collect();
    assert_eq!(folders, vec![(rust, 2), (reading, 1)]);
    let tags: Vec<(&str, i64)> = s.tags.iter().map(|t| (t.name.as_str(), t.count)).collect();
    assert_eq!(tags, vec![("lang", 2), ("systems", 1)]);
}

#[tokio::test]
async fn nothing_is_suggested_for_someone_elses_bookmark_or_a_lone_site() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let them = person(&pool).await;
    let lone = bookmark(
        &pool,
        &me,
        &format!("https://{}.example/", Uuid::new_v4().simple()),
        vec![],
        &[],
    )
    .await;
    let s = db::filing_suggestions(&pool, me.id, lone)
        .await
        .expect("suggest");
    assert!(s.folders.is_empty() && s.tags.is_empty());

    assert!(db::filing_suggestions(&pool, them.id, lone).await.is_err());
}
