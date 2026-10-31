//! Authz regression: user A cannot read/write user B's rows.
//!
//! Requires Postgres. Set `DATABASE_URL` (e.g. `postgres://linkdean:linkdean@127.0.0.1:5432/linkdean`).
//! Skips when unset so CI without a DB stays green.

use linkdean_core::{
    CreateBookmark, CreateCategory, Error, UpdateBookmark, UpdateCategory,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping authz tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for authz tests");
    db::migrate(&pool)
        .await
        .expect("migrate for authz tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@authz.test", Uuid::new_v4())
}

#[tokio::test]
async fn user_b_cannot_access_user_a_bookmark_or_category() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("a"),
        Some("password-a-long"),
        Some("User A"),
        None,
        None,
    )
    .await
    .expect("create user a");
    let user_b = db::create_user(
        &pool,
        &unique_email("b"),
        Some("password-b-long"),
        Some("User B"),
        None,
        None,
    )
    .await
    .expect("create user b");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("AuthzCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
        },
    )
    .await
    .expect("create category for a");

    let bm = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://authz.example/{}/", Uuid::new_v4()),
            title: Some("Owned by A".into()),
            description: None,
            notes: None,
            category_id: Some(cat.id),
            category_ids: vec![cat.id],
            tags: vec![],
        },
        None,
    )
    .await
    .expect("create bookmark for a");

    // A can read own rows
    assert!(db::get_bookmark_raw(&pool, user_a.id, bm.id).await.is_ok());
    assert!(db::get_category(&pool, user_a.id, cat.id).await.is_ok());

    // B cannot get / update / delete A's bookmark
    assert!(matches!(
        db::get_bookmark_raw(&pool, user_b.id, bm.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::update_bookmark(
            &pool,
            user_b.id,
            bm.id,
            UpdateBookmark {
                url: Some("https://evil.example/".into()),
                title: None,
                description: None,
                notes: None,
                category_id: None,
                category_ids: None,
                tags: None,
            },
            None,
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::delete_bookmark(&pool, user_b.id, bm.id).await,
        Err(Error::NotFound)
    ));

    // B cannot get / update / delete A's category
    assert!(matches!(
        db::get_category(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::update_category(
            &pool,
            user_b.id,
            cat.id,
            UpdateCategory {
                name: Some("hijacked".into()),
                parent_id: None,
                is_hidden: None,
                position: None,
            },
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::delete_category(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound)
    ));

    // B's list never includes A's bookmark
    let b_list = db::list_bookmarks(&pool, user_b.id, None, false, None)
        .await
        .expect("list for b");
    assert!(!b_list.iter().any(|x| x.id == bm.id));

    // Cleanup A's rows (best-effort; unique emails avoid collisions)
    let _ = db::delete_bookmark(&pool, user_a.id, bm.id).await;
    let _ = db::delete_category(&pool, user_a.id, cat.id).await;
}
