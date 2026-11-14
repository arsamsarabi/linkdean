//! Scheduling, cancelling and purging an account.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, Error, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::AssertSqlSafe;
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

/// A signed-in session for `user_id` that just signed in again with its provider; returns the
/// session token hash.
async fn step_up(pool: &PgPool, user_id: Uuid) -> String {
    let token = db::create_session(pool, user_id, 1).await.expect("session");
    let hash = linkdean_core::crypto::hash_token(&token);
    db::mark_session_reauthenticated(pool, user_id, &hash)
        .await
        .expect("step up");
    hash
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

async fn count(pool: &PgPool, sql: &'static str, id: Uuid) -> i64 {
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
    let err = db::schedule_account_deletion(
        &pool,
        user.id,
        "someone@else.test",
        None,
        Some(&step_up(&pool, user.id).await),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "{err}");
    assert_eq!(scheduled_at(&pool, user.id).await, None);

    // Case and surrounding spaces do not matter.
    let confirm = format!(" {} ", user.email.to_uppercase());
    let at = db::schedule_account_deletion(
        &pool,
        user.id,
        &confirm,
        None,
        Some(&step_up(&pool, user.id).await),
    )
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
    let none = db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        None,
        Some(&step_up(&pool, user.id).await),
    )
    .await
    .unwrap_err();
    assert!(matches!(none, Error::BadRequest(_)), "{none}");
    let wrong = db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        Some("nope-nope".into()),
        Some(&step_up(&pool, user.id).await),
    )
    .await
    .unwrap_err();
    assert!(matches!(wrong, Error::Unauthorized), "{wrong}");
    assert_eq!(scheduled_at(&pool, user.id).await, None);

    db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        Some(PASSWORD.into()),
        Some(&step_up(&pool, user.id).await),
    )
    .await
    .expect("schedule");
    assert!(scheduled_at(&pool, user.id).await.is_some());
}

#[tokio::test]
async fn a_single_sign_on_account_must_sign_in_again_first() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, None).await;
    let token = db::create_session(&pool, user.id, 1)
        .await
        .expect("session");
    let hash = linkdean_core::crypto::hash_token(&token);

    // No session (an API key) and a session that never signed in again are both refused.
    for session in [None, Some(hash.as_str())] {
        let err = db::schedule_account_deletion(&pool, user.id, &user.email, None, session)
            .await
            .unwrap_err();
        assert!(
            matches!(&err, Error::BadRequest(m) if m == db::REAUTH_REQUIRED),
            "{err}"
        );
    }
    // Another account's session cannot step up for this one.
    let other = person(&pool, None).await;
    let err = db::mark_session_reauthenticated(&pool, other.id, &hash)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unauthorized), "{err}");

    // A step-up older than the window no longer counts.
    db::mark_session_reauthenticated(&pool, user.id, &hash)
        .await
        .expect("step up");
    sqlx::query(
        "UPDATE sessions SET reauthenticated_at = now() - interval '11 minutes' \
         WHERE token_hash = $1",
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .expect("age");
    assert!(!db::session_recently_reauthenticated(&pool, user.id, &hash)
        .await
        .expect("check"));
    assert_eq!(scheduled_at(&pool, user.id).await, None);

    db::mark_session_reauthenticated(&pool, user.id, &hash)
        .await
        .expect("step up");
    db::schedule_account_deletion(&pool, user.id, &user.email, None, Some(&hash))
        .await
        .expect("schedule");
    assert!(scheduled_at(&pool, user.id).await.is_some());
}

#[tokio::test]
async fn a_single_sign_on_account_ignores_a_password() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let user = person(&pool, None).await;
    // A password sent along is ignored, not checked.
    db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        Some("ignored".into()),
        Some(&step_up(&pool, user.id).await),
    )
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
    let first = db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        None,
        Some(&step_up(&pool, user.id).await),
    )
    .await
    .expect("schedule");
    let again = db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        None,
        Some(&step_up(&pool, user.id).await),
    )
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
    db::schedule_account_deletion(
        &pool,
        user.id,
        &user.email,
        None,
        Some(&step_up(&pool, user.id).await),
    )
    .await
    .expect("schedule");

    // Not due yet: nothing is deleted.
    assert!(!db::list_users_due_for_deletion(&pool)
        .await
        .expect("due")
        .contains(&user.id));
    assert_eq!(
        db::purge_account(&pool, user.id)
            .await
            .expect("early purge"),
        db::PurgeOutcome::NotDue
    );
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
    assert_eq!(
        db::purge_account(&pool, user.id)
            .await
            .expect("cancelled purge"),
        db::PurgeOutcome::NotDue
    );

    make_due(&pool, user.id).await;
    let outcome = db::purge_account(&pool, user.id).await.expect("purge");
    assert_eq!(outcome, db::PurgeOutcome::Purged(vec![bookmark]));
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
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
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
    let err = db::schedule_account_deletion(
        &pool,
        first.id,
        &first.email,
        None,
        Some(&step_up(&pool, first.id).await),
    )
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
    db::schedule_account_deletion(
        &pool,
        first.id,
        &first.email,
        None,
        Some(&step_up(&pool, first.id).await),
    )
    .await
    .expect("another admin exists");

    // Both admins schedule while the other is there. At purge time only one may go: the
    // second stays, still scheduled, until another admin exists.
    db::schedule_account_deletion(
        &pool,
        second.id,
        &second.email,
        None,
        Some(&step_up(&pool, second.id).await),
    )
    .await
    .expect("the first admin still exists");
    make_due(&pool, first.id).await;
    make_due(&pool, second.id).await;
    assert!(matches!(
        db::purge_account(&pool, first.id)
            .await
            .expect("purge first"),
        db::PurgeOutcome::Purged(_)
    ));
    assert_eq!(
        db::purge_account(&pool, second.id)
            .await
            .expect("purge second"),
        db::PurgeOutcome::LastAdmin
    );
    assert!(scheduled_at(&pool, second.id).await.is_some());

    let third = db::create_user(&pool, "third@deletion.test", None, None, None, None)
        .await
        .expect("third");
    sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1")
        .bind(third.id)
        .execute(&pool)
        .await
        .expect("promote third");
    assert!(matches!(
        db::purge_account(&pool, second.id)
            .await
            .expect("purge second again"),
        db::PurgeOutcome::Purged(_)
    ));

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
    )))
    .execute(&mut admin_conn)
    .await
    .expect("drop database");
}
