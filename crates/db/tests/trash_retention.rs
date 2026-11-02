//! Trash retention stays in a range the purge can compute.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{Error, UpdateSettings};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping trash retention tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn retention(days: i32) -> UpdateSettings {
    UpdateSettings {
        show_hidden: None,
        vault_search_mode: None,
        theme: None,
        vault_auto_lock_minutes: None,
        default_landing: None,
        library_view: None,
        trash_retention_days: Some(days),
        show_deleted_in_library: None,
        show_vault_in_library: None,
        vault_delete_mode: None,
    }
}

#[tokio::test]
async fn retention_outside_the_range_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("retention-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");

    for days in [0, 3651, i32::MAX] {
        let err = db::update_user_settings(&pool, user.id, &retention(days))
            .await
            .expect_err("out of range");
        assert!(matches!(err, Error::BadRequest(_)), "{days} gave {err:?}");
    }

    let saved = db::update_user_settings(&pool, user.id, &retention(3650))
        .await
        .expect("top of the range");
    assert_eq!(saved.trash_retention_days, 3650);
}

#[tokio::test]
async fn the_database_refuses_a_retention_outside_the_range() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("retention-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    db::update_user_settings(&pool, user.id, &retention(30))
        .await
        .expect("settings row");

    let written =
        sqlx::query("UPDATE user_settings SET trash_retention_days = $1 WHERE user_id = $2")
            .bind(i32::MAX)
            .bind(user.id)
            .execute(&pool)
            .await;
    assert!(written.is_err(), "a CHECK must refuse the value");
}

#[tokio::test]
async fn purge_returns_bookmark_ids_and_drops_their_social_rows() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("purge-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    let old = db::create_bookmark(
        &pool,
        user.id,
        linkdean_core::CreateBookmark {
            url: format!("https://purge.example/{}", Uuid::new_v4()),
            ..Default::default()
        },
        &linkdean_core::VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id;
    sqlx::query("UPDATE bookmarks SET deleted_at = now() - interval '40 days' WHERE id = $1")
        .bind(old)
        .execute(&pool)
        .await
        .expect("age");
    sqlx::query(
        "INSERT INTO social_likes (user_id, subject_type, subject_id) VALUES ($1, 'bookmark', $2)",
    )
    .bind(user.id)
    .bind(old)
    .execute(&pool)
    .await
    .expect("like");

    let purged = db::purge_expired_trash(&pool).await.expect("purge");
    assert!(purged.bookmark_ids.contains(&old));

    let likes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM social_likes WHERE subject_id = $1")
        .bind(old)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(likes, 0);
}

#[tokio::test]
async fn expired_sessions_are_purged() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("sessions-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    let live = db::create_session(&pool, user.id, 7).await.expect("live");
    db::create_session(&pool, user.id, -1)
        .await
        .expect("expired");

    assert!(db::purge_expired_sessions(&pool).await.expect("purge") >= 1);

    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(left, 1);
    assert!(db::user_from_token(&pool, &live)
        .await
        .expect("lookup")
        .is_some());
}

#[tokio::test]
async fn the_library_view_accepts_list_and_refuses_unknown_layouts() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("view-{}@limits.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    let view = |v: &str| UpdateSettings {
        library_view: Some(v.into()),
        trash_retention_days: None,
        ..retention(30)
    };
    for v in ["list", "explorer", "grid"] {
        let saved = db::update_user_settings(&pool, user.id, &view(v))
            .await
            .expect(v);
        assert_eq!(saved.library_view, v);
    }
    let err = db::update_user_settings(&pool, user.id, &view("table"))
        .await
        .expect_err("unknown layout");
    assert!(matches!(err, Error::BadRequest(_)));
}
