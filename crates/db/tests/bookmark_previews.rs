//! Which bookmarks an import queues for enrichment.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
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
        &VaultKeyring::default(),
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

#[tokio::test]
async fn the_boot_backfill_skips_trash() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "backfill").await;
    let live = bookmark(&pool, &owner).await;
    let trashed = bookmark(&pool, &owner).await;
    sqlx::query("UPDATE bookmarks SET deleted_at = now() WHERE id = $1")
        .bind(trashed)
        .execute(&pool)
        .await
        .expect("trash");

    let rows = db::list_bookmarks_needing_preview(&pool, 7)
        .await
        .expect("list");
    assert!(rows.contains(&(owner.id, live)));
    assert!(!rows.contains(&(owner.id, trashed)));
}

async fn failed_hours_ago(pool: &PgPool, id: Uuid, hours: i32) {
    sqlx::query(
        "UPDATE bookmarks SET preview_failed_at = now() - make_interval(hours => $2) WHERE id = $1",
    )
    .bind(id)
    .bind(hours)
    .execute(pool)
    .await
    .expect("mark failed");
}

#[tokio::test]
async fn failed_screenshots_are_retried_only_once_the_cooldown_has_passed() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "retry").await;
    let old = bookmark(&pool, &owner).await;
    let fresh = bookmark(&pool, &owner).await;
    let never_tried = bookmark(&pool, &owner).await;
    let removed = bookmark(&pool, &owner).await;
    failed_hours_ago(&pool, old, 30).await;
    failed_hours_ago(&pool, fresh, 2).await;
    failed_hours_ago(&pool, removed, 30).await;
    db::set_bookmark_has_preview(&pool, removed, false)
        .await
        .expect("remove preview");
    // Removing the preview clears the failure; mark it failed again to show it is still skipped.
    failed_hours_ago(&pool, removed, 30).await;

    let rows = db::list_failed_previews(&pool, 24).await.expect("list");
    assert!(rows.contains(&(owner.id, old)));
    assert!(!rows.contains(&(owner.id, fresh)));
    assert!(!rows.contains(&(owner.id, never_tried)));
    assert!(!rows.contains(&(owner.id, removed)));

    let n = db::count_failed_previews(&pool, 24).await.expect("count");
    assert!(n >= 1);
    assert_eq!(n as usize, rows.len());
}

#[tokio::test]
async fn the_automatic_retry_window_is_configurable() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "window").await;
    let two_days = bookmark(&pool, &owner).await;
    failed_hours_ago(&pool, two_days, 48).await;

    let week = db::list_bookmarks_needing_preview(&pool, 7)
        .await
        .expect("list");
    assert!(!week.contains(&(owner.id, two_days)));
    let one_day = db::list_bookmarks_needing_preview(&pool, 1)
        .await
        .expect("list");
    assert!(one_day.contains(&(owner.id, two_days)));
}

#[tokio::test]
async fn a_manual_retry_can_be_claimed_once_per_cooldown() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    // The settings row is shared; only this test changes the retry columns.
    let s = db::set_screenshot_retry_settings(&pool, Some(24), Some(7))
        .await
        .expect("settings");
    assert_eq!(
        (
            s.screenshot_retry_cooldown_hours,
            s.screenshot_auto_retry_days
        ),
        (24, 7)
    );
    sqlx::query("UPDATE platform_settings SET screenshot_retry_last_run_at = NULL")
        .execute(&pool)
        .await
        .expect("reset");

    let first = db::claim_screenshot_retry(&pool).await.expect("claim");
    assert!(first.is_some());
    assert!(db::claim_screenshot_retry(&pool)
        .await
        .expect("claim")
        .is_none());

    sqlx::query(
        "UPDATE platform_settings SET screenshot_retry_last_run_at = now() - interval '25 hours'",
    )
    .execute(&pool)
    .await
    .expect("age");
    assert!(db::claim_screenshot_retry(&pool)
        .await
        .expect("claim")
        .is_some());

    assert!(db::set_screenshot_retry_settings(&pool, Some(0), None)
        .await
        .is_err());
}
