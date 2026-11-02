//! Link checks leave `updated_at` alone, and unused tags are reaped.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, LinkCheckStatus, UpdateBookmark, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping housekeeping tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("hk-{}@housekeeping.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn bookmark(pool: &PgPool, owner: &User, tags: Vec<String>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://hk.example/{}", Uuid::new_v4()),
            title: Some("Kept".into()),
            tags,
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn link_checks_do_not_touch_updated_at() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let b = bookmark(&pool, &owner, vec![]).await;
    let updated_at = |pool: PgPool| async move {
        sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
            "SELECT updated_at FROM bookmarks WHERE id = $1",
        )
        .bind(b)
        .fetch_one(&pool)
        .await
        .expect("updated_at")
    };
    let before = updated_at(pool.clone()).await;
    db::record_link_check(&pool, b, LinkCheckStatus::Ok, Some(200), None)
        .await
        .expect("check");
    assert_eq!(updated_at(pool.clone()).await, before);
}

#[tokio::test]
async fn unused_tags_are_reaped_after_an_hour() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    bookmark(&pool, &owner, vec!["used".into()]).await;
    db::ensure_tags(&pool, owner.id, &["old-unused".into(), "new-unused".into()])
        .await
        .expect("tags");
    sqlx::query(
        "UPDATE tags SET created_at = now() - interval '2 hours' WHERE owner_user_id = $1 AND name <> 'new-unused'",
    )
    .bind(owner.id)
    .execute(&pool)
    .await
    .expect("age tags");

    db::reap_unused_tags(&pool).await.expect("reap");
    let mut left: Vec<String> = db::list_tags(&pool, owner.id)
        .await
        .expect("tags")
        .into_iter()
        .map(|t| t.name)
        .collect();
    left.sort();
    assert_eq!(left, vec!["new-unused".to_string(), "used".to_string()]);
}

async fn wanted(pool: &PgPool, owner: &User, id: Uuid) -> bool {
    db::list_user_bookmarks_needing_preview(pool, owner.id)
        .await
        .expect("needing")
        .iter()
        .any(|(_, b)| *b == id)
}

#[tokio::test]
async fn a_removed_or_failed_preview_is_not_retaken() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let b = bookmark(&pool, &owner, vec![]).await;
    assert!(wanted(&pool, &owner, b).await);

    db::set_bookmark_has_preview(&pool, b, true)
        .await
        .expect("saved");
    db::set_bookmark_has_preview(&pool, b, false)
        .await
        .expect("removed");
    assert!(!wanted(&pool, &owner, b).await);
    assert!(!db::bookmark_wants_preview(&pool, b).await.expect("wants"));

    db::update_bookmark(
        &pool,
        owner.id,
        b,
        UpdateBookmark {
            url: Some(format!("https://moved-{}.example/", Uuid::new_v4())),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("new url");
    assert!(wanted(&pool, &owner, b).await);

    db::record_preview_failure(&pool, b).await.expect("failed");
    assert!(!wanted(&pool, &owner, b).await);
    sqlx::query("UPDATE bookmarks SET preview_failed_at = now() - interval '8 days' WHERE id = $1")
        .bind(b)
        .execute(&pool)
        .await
        .expect("age");
    assert!(wanted(&pool, &owner, b).await);
}

#[tokio::test]
async fn a_refused_edit_changes_nothing() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let b = bookmark(&pool, &owner, vec!["keep".into()]).await;

    let nul = db::update_bookmark(
        &pool,
        owner.id,
        b,
        UpdateBookmark {
            title: Some("New title".into()),
            tags: Some(vec!["ok".into(), "bad\0tag".into()]),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(nul, Err(linkdean_core::Error::BadRequest(_))));

    let missing_folder = db::update_bookmark(
        &pool,
        owner.id,
        b,
        UpdateBookmark {
            title: Some("New title".into()),
            category_ids: Some(vec![Uuid::new_v4()]),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(missing_folder.is_err());

    let raw = db::get_bookmark_raw(&pool, owner.id, b).await.expect("raw");
    assert_ne!(raw.title, "New title");
    let view = db::bookmark_to_view(&pool, raw, &VaultKeyring::default(), owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.tags, vec!["keep".to_string()]);
}

#[tokio::test]
async fn concurrent_edits_to_different_fields_both_land() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    for round in 0..5 {
        let b = bookmark(&pool, &owner, vec![]).await;
        let title = format!("Title {round}");
        let notes = format!("Notes {round}");
        let edit = |req: UpdateBookmark| {
            let pool = pool.clone();
            let owner_id = owner.id;
            async move {
                db::update_bookmark(&pool, owner_id, b, req, &VaultKeyring::default())
                    .await
                    .expect("edit")
            }
        };
        tokio::join!(
            edit(UpdateBookmark {
                title: Some(title.clone()),
                ..Default::default()
            }),
            edit(UpdateBookmark {
                notes: Some(Some(notes.clone())),
                ..Default::default()
            }),
        );
        let raw = db::get_bookmark_raw(&pool, owner.id, b).await.expect("raw");
        assert_eq!((raw.title, raw.notes), (title, Some(notes)));
    }
}

#[tokio::test]
async fn adding_tags_and_folders_keeps_what_is_there() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let folder = |name: &'static str| {
        let pool = pool.clone();
        let owner_id = owner.id;
        async move {
            db::create_category(
                &pool,
                owner_id,
                linkdean_core::CreateCategory {
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
    };
    let (first, second) = (folder("First").await, folder("Second").await);
    let b = bookmark(&pool, &owner, vec!["old".into()]).await;
    db::update_bookmark(
        &pool,
        owner.id,
        b,
        UpdateBookmark {
            category_ids: Some(vec![first]),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("file");

    let view = db::update_bookmark(
        &pool,
        owner.id,
        b,
        UpdateBookmark {
            add_tags: Some(vec!["new".into(), "old".into()]),
            add_category_ids: Some(vec![second]),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("add");
    let mut tags = view.tags.clone();
    tags.sort();
    assert_eq!(tags, vec!["new".to_string(), "old".to_string()]);
    let mut folders: Vec<Uuid> = view.categories.iter().map(|c| c.id).collect();
    folders.sort();
    let mut want = vec![first, second];
    want.sort();
    assert_eq!(folders, want);
}
