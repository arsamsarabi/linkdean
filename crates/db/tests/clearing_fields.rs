//! Patches can clear optional fields.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, UpdateBookmark, UpdateProfile, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping clearing tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

#[tokio::test]
async fn a_bookmark_description_and_notes_can_be_cleared() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("clearing-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    let keyring = VaultKeyring::default();
    let bookmark = db::create_bookmark(
        &pool,
        user.id,
        CreateBookmark {
            url: "https://example.com/".into(),
            description: Some("words".into()),
            notes: Some("more words".into()),
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("bookmark");

    let cleared = db::update_bookmark(
        &pool,
        user.id,
        bookmark.id,
        UpdateBookmark {
            url: None,
            title: None,
            description: Some(None),
            notes: Some(None),
            category_id: None,
            category_ids: None,
            tags: None,
            is_favorite: None,
            is_unread: None,
            is_pinned: None,
            is_private: None,
        },
        &keyring,
    )
    .await
    .expect("clear");
    assert_eq!(cleared.description, None);
    assert_eq!(cleared.notes, None);
}

#[tokio::test]
async fn a_profile_field_can_be_cleared() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("clearing-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(
        &pool,
        &email,
        Some("password-long"),
        Some("Name"),
        None,
        None,
    )
    .await
    .expect("user");

    let patch = UpdateProfile {
        display_name: Some(None),
        bio: None,
        timezone: None,
        email: None,
        profile_visibility: None,
    };
    let updated = db::update_profile(&pool, user.id, &patch)
        .await
        .expect("clear");
    assert_eq!(updated.display_name, None);
}
