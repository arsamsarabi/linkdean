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
