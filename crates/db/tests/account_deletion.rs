//! Scheduling, cancelling and purging an account.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, Error, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

const PASSWORD: &str = "password-long";

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping account deletion tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

/// A user who is not an admin, so the last-admin rule never depends on what else is in the
/// shared database.
async fn person(pool: &PgPool, password: Option<&str>) -> User {
    let email = format!("gone-{}@deletion.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, password, None, None, None)
        .await
        .expect("user");
    sqlx::query("UPDATE users SET is_admin = FALSE WHERE id = $1")
        .bind(user.id)
        .execute(pool)
        .await
        .expect("demote");
    user
}

async fn scheduled_at(pool: &PgPool, id: Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar("SELECT deletion_scheduled_at FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("scheduled_at")
}

async fn make_due(pool: &PgPool, id: Uuid) {
    sqlx::query(
        "UPDATE users SET deletion_scheduled_at = now() - interval '1 minute' WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("make due");
}

async fn count(pool: &PgPool, sql: &str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn scheduling_needs_the_matching_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, None).await;
    let err = db::schedule_account_deletion(&pool, user.id, "someone@else.test", None)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "{err}");
    assert_eq!(scheduled_at(&pool, user.id).await, None);

    // Case and surrounding spaces do not matter.
    let confirm = format!(" {} ", user.email.to_uppercase());
    let at = db::schedule_account_deletion(&pool, user.id, &confirm, None)
        .await
        .expect("schedule");
    let days = (at - chrono::Utc::now()).num_hours();
    assert!(
        (days - db::ACCOUNT_DELETION_GRACE_DAYS * 24).abs() <= 1,
        "{days}h"
    );
    assert_eq!(scheduled_at(&pool, user.id).await, Some(at));
}

#[tokio::test]
async fn a_password_account_must_give_its_password() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, Some(PASSWORD)).await;
    let none = db::schedule_account_deletion(&pool, user.id, &user.email, None)
        .await
        .unwrap_err();
    assert!(matches!(none, Error::BadRequest(_)), "{none}");
    let wrong =
        db::schedule_account_deletion(&pool, user.id, &user.email, Some("nope-nope".into()))
            .await
            .unwrap_err();
    assert!(matches!(wrong, Error::Unauthorized), "{wrong}");
    assert_eq!(scheduled_at(&pool, user.id).await, None);

    db::schedule_account_deletion(&pool, user.id, &user.email, Some(PASSWORD.into()))
        .await
        .expect("schedule");
    assert!(scheduled_at(&pool, user.id).await.is_some());
}

#[tokio::test]
async fn a_single_sign_on_account_needs_only_the_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, None).await;
    // A password sent along is ignored, not checked.
    db::schedule_account_deletion(&pool, user.id, &user.email, Some("ignored".into()))
        .await
        .expect("schedule");
    assert!(scheduled_at(&pool, user.id).await.is_some());
}

#[tokio::test]
async fn scheduling_twice_keeps_the_first_date_and_cancel_clears_it() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, None).await;
    let first = db::schedule_account_deletion(&pool, user.id, &user.email, None)
        .await
        .expect("schedule");
    let again = db::schedule_account_deletion(&pool, user.id, &user.email, None)
        .await
        .expect("schedule again");
    assert_eq!(first, again);

    db::cancel_account_deletion(&pool, user.id)
        .await
        .expect("cancel");
    assert_eq!(scheduled_at(&pool, user.id).await, None);
    let err = db::cancel_account_deletion(&pool, user.id)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "{err}");
}

#[tokio::test]
async fn purge_removes_the_user_and_returns_their_bookmark_ids() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, None).await;
    let bookmark = db::create_bookmark(
        &pool,
        user.id,
        CreateBookmark {
            url: format!("https://deletion.example/{}", Uuid::new_v4()),
            title: Some("Doomed".into()),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id;
    db::schedule_account_deletion(&pool, user.id, &user.email, None)
        .await
        .expect("schedule");

    // Not due yet: nothing is deleted.
    assert!(!db::list_users_due_for_deletion(&pool)
        .await
        .expect("due")
        .contains(&user.id));
    assert!(db::purge_account(&pool, user.id)
        .await
        .expect("early purge")
        .is_empty());
    assert!(db::find_user_by_id(&pool, user.id)
        .await
        .expect("find")
        .is_some());

    // A cancelled account is not purged even if it was listed as due.
    make_due(&pool, user.id).await;
    assert!(db::list_users_due_for_deletion(&pool)
        .await
        .expect("due")
        .contains(&user.id));
    db::cancel_account_deletion(&pool, user.id)
        .await
        .expect("cancel");
    assert!(db::purge_account(&pool, user.id)
        .await
        .expect("cancelled purge")
        .is_empty());

    make_due(&pool, user.id).await;
    let ids = db::purge_account(&pool, user.id).await.expect("purge");
    assert_eq!(ids, vec![bookmark]);
    assert!(db::find_user_by_id(&pool, user.id)
        .await
        .expect("find")
        .is_none());
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM bookmarks WHERE id = $1",
            bookmark
        )
        .await,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM user_settings WHERE user_id = $1",
            user.id
        )
        .await,
        0
    );
}

/// "The last admin" is instance-wide, so this test uses its own new database.
#[tokio::test]
async fn the_last_admin_cannot_schedule_deletion() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping account deletion tests: DATABASE_URL unset");
        return;
    };
    let name = format!("deletion_{}", Uuid::new_v4().simple());
    let mut admin_conn = PgConnection::connect(&url).await.expect("admin conn");
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&mut admin_conn)
        .await
        .expect("create database");
    let base = url.rsplit_once('/').expect("database in url").0;
    let pool = db::connect(&format!("{base}/{name}"))
        .await
        .expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // The first account is the admin.
    let first = db::create_user(&pool, "first@deletion.test", None, None, None, None)
        .await
        .expect("first");
    assert!(first.is_admin);
    let err = db::schedule_account_deletion(&pool, first.id, &first.email, None)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "{err}");
    assert_eq!(scheduled_at(&pool, first.id).await, None);

    let second = db::create_user(&pool, "second@deletion.test", None, None, None, None)
        .await
        .expect("second");
    sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1")
        .bind(second.id)
        .execute(&pool)
        .await
        .expect("promote");
    db::schedule_account_deletion(&pool, first.id, &first.email, None)
        .await
        .expect("another admin exists");

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
        .execute(&mut admin_conn)
        .await
        .expect("drop database");
}
