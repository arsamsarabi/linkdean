//! Saved search CRUD. Requires Postgres (`DATABASE_URL`).

use linkdean_core::{CreateSavedSearch, Error, PatchSavedSearch, SavedSearchFilters};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping saved_searches tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for saved_searches tests");
    db::migrate(&pool)
        .await
        .expect("migrate for saved_searches tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@savedsearch.test", Uuid::new_v4())
}

#[tokio::test]
async fn create_list_patch_delete_and_owner_isolation() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let a = db::create_user(
        &pool,
        &unique_email("a"),
        Some("password-long-enough"),
        Some("A"),
        None,
        None,
    )
    .await
    .expect("user a");
    let b = db::create_user(
        &pool,
        &unique_email("b"),
        Some("password-long-enough"),
        Some("B"),
        None,
        None,
    )
    .await
    .expect("user b");

    let created = db::create_saved_search(
        &pool,
        a.id,
        CreateSavedSearch {
            name: "  Favorites recent  ".into(),
            filters: SavedSearchFilters {
                flag: vec!["favorite".into()],
                q: Some("rust".into()),
                ..Default::default()
            },
        },
    )
    .await
    .expect("create");
    assert_eq!(created.name, "Favorites recent");
    assert_eq!(created.filters.q.as_deref(), Some("rust"));
    assert_eq!(created.filters.flag, vec!["favorite"]);

    let listed = db::list_saved_searches(&pool, a.id).await.expect("list a");
    assert!(listed.iter().any(|s| s.id == created.id));
    assert!(db::list_saved_searches(&pool, b.id)
        .await
        .expect("list b")
        .is_empty());

    let dup = db::create_saved_search(
        &pool,
        a.id,
        CreateSavedSearch {
            name: "favorites recent".into(),
            filters: SavedSearchFilters::default(),
        },
    )
    .await;
    assert!(matches!(dup, Err(Error::Conflict(_))));

    let patched = db::patch_saved_search(
        &pool,
        a.id,
        created.id,
        PatchSavedSearch {
            name: Some("Renamed".into()),
            filters: Some(SavedSearchFilters {
                tag: vec!["docs".into()],
                ..Default::default()
            }),
        },
    )
    .await
    .expect("patch");
    assert_eq!(patched.name, "Renamed");
    assert_eq!(patched.filters.tag, vec!["docs"]);
    assert!(patched.filters.flag.is_empty());

    let steal = db::delete_saved_search(&pool, b.id, created.id).await;
    assert!(matches!(steal, Err(Error::NotFound)));

    db::delete_saved_search(&pool, a.id, created.id)
        .await
        .expect("delete");
    assert!(db::list_saved_searches(&pool, a.id)
        .await
        .expect("list after")
        .iter()
        .all(|s| s.id != created.id));
}
