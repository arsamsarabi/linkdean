//! The email outbox: claiming, retries, giving up, stats and retention.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset. Each test runs in its own new
//! database, because claiming takes every due row in the instance.

use linkdean_db::{self as db, EmailRetry, NewOutboxEmail};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

struct FreshDb {
    pool: PgPool,
    admin_url: String,
    name: String,
}

impl FreshDb {
    async fn drop_db(self) {
        self.pool.close().await;
        let mut admin = PgConnection::connect(&self.admin_url).await.expect("admin");
        sqlx::query(&format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.name
        ))
        .execute(&mut admin)
        .await
        .expect("drop database");
    }
}

async fn fresh_db_or_skip() -> Option<FreshDb> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping email outbox tests: DATABASE_URL unset");
        return None;
    };
    let name = format!("mail_{}", Uuid::new_v4().simple());
    let mut admin = PgConnection::connect(&url).await.expect("admin");
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&mut admin)
        .await
        .expect("create database");
    let base = url.rsplit_once('/').expect("database in url").0;
    let pool = db::connect(&format!("{base}/{name}"))
        .await
        .expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(FreshDb {
        pool,
        admin_url: url,
        name,
    })
}

fn email(to: &str) -> NewOutboxEmail {
    NewOutboxEmail {
        recipient: to.into(),
        template: "test".into(),
        subject: "Hello".into(),
        html: "<p>Hi</p>".into(),
        text: "Hi".into(),
    }
}

#[tokio::test]
async fn two_senders_never_claim_the_same_email() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = fresh.pool.clone();
    for i in 0..10 {
        db::enqueue_email(&pool, &email(&format!("u{i}@example.com")))
            .await
            .unwrap();
    }
    let (a, b) = tokio::join!(
        db::claim_due_emails(&pool, 10),
        db::claim_due_emails(&pool, 10)
    );
    let mut ids: Vec<Uuid> = a
        .unwrap()
        .into_iter()
        .chain(b.unwrap())
        .map(|e| e.id)
        .collect();
    assert_eq!(ids.len(), 10, "every email claimed once");
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10, "no email claimed twice");
    // A claimed email is not handed out again while its claim lasts.
    assert!(db::claim_due_emails(&pool, 10).await.unwrap().is_empty());
    fresh.drop_db().await;
}

#[tokio::test]
async fn a_sent_email_is_done() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = fresh.pool.clone();
    let id = db::enqueue_email(&pool, &email("a@example.com"))
        .await
        .unwrap();
    let claimed = db::claim_due_emails(&pool, 5).await.unwrap();
    assert_eq!(claimed[0].id, id);
    assert_eq!(claimed[0].recipient, "a@example.com");
    db::mark_email_sent(&pool, id).await.unwrap();
    sqlx::query(
        "UPDATE email_outbox SET claimed_until = NULL, next_attempt_at = now() - interval '1 hour'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(db::claim_due_emails(&pool, 5).await.unwrap().is_empty());
    let stats = db::email_stats(&pool).await.unwrap();
    assert_eq!((stats.sent, stats.queued, stats.failed), (1, 0, 0));
    fresh.drop_db().await;
}

#[tokio::test]
async fn a_failure_backs_off_then_gives_up_after_a_day() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = fresh.pool.clone();
    let id = db::enqueue_email(&pool, &email("b@example.com"))
        .await
        .unwrap();
    db::claim_due_emails(&pool, 5).await.unwrap();

    let retry = db::record_email_failure(&pool, id, "connection refused")
        .await
        .unwrap();
    assert_eq!(retry, EmailRetry::Retrying);
    let (delay,): (f64,) = sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM next_attempt_at - now())::float8 FROM email_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        (25.0..=31.0).contains(&delay),
        "first retry after about 30 s, got {delay}"
    );
    // Not due yet.
    assert!(db::claim_due_emails(&pool, 5).await.unwrap().is_empty());

    // Later attempts double, capped at an hour.
    sqlx::query("UPDATE email_outbox SET attempts = 9 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    db::record_email_failure(&pool, id, "connection refused")
        .await
        .unwrap();
    let (delay,): (f64,) = sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM next_attempt_at - now())::float8 FROM email_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        (3590.0..=3601.0).contains(&delay),
        "capped at an hour, got {delay}"
    );

    // A day after it was queued, the next failure is final.
    sqlx::query("UPDATE email_outbox SET created_at = now() - interval '25 hours' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let retry = db::record_email_failure(&pool, id, "mailbox unavailable")
        .await
        .unwrap();
    assert_eq!(retry, EmailRetry::GaveUp);
    let stats = db::email_stats(&pool).await.unwrap();
    assert_eq!((stats.queued, stats.failed), (0, 1));
    assert_eq!(stats.last_error.as_deref(), Some("mailbox unavailable"));
    assert!(stats.last_error_at.is_some());
    fresh.drop_db().await;
}

#[tokio::test]
async fn a_crashed_senders_claim_expires() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = fresh.pool.clone();
    let id = db::enqueue_email(&pool, &email("c@example.com"))
        .await
        .unwrap();
    db::claim_due_emails(&pool, 5).await.unwrap();
    sqlx::query(
        "UPDATE email_outbox SET claimed_until = now() - interval '1 second' WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    let again = db::claim_due_emails(&pool, 5).await.unwrap();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].id, id);
    fresh.drop_db().await;
}

#[tokio::test]
async fn old_rows_are_purged_by_status() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = fresh.pool.clone();
    let mut ids = Vec::new();
    for to in [
        "sent-old",
        "sent-new",
        "failed-old",
        "failed-mid",
        "queued-old",
    ] {
        ids.push(
            db::enqueue_email(&pool, &email(&format!("{to}@example.com")))
                .await
                .unwrap(),
        );
    }
    let set = |id: Uuid, status: &'static str, days: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query("UPDATE email_outbox SET status = $2, updated_at = now() - make_interval(days => $3) WHERE id = $1")
                .bind(id)
                .bind(status)
                .bind(days)
                .execute(&pool)
                .await
                .unwrap();
        }
    };
    set(ids[0], "sent", 8).await;
    set(ids[1], "sent", 1).await;
    set(ids[2], "failed", 31).await;
    set(ids[3], "failed", 8).await;
    set(ids[4], "queued", 40).await;

    assert_eq!(db::purge_old_emails(&pool).await.unwrap(), 2);
    let (left,): (i64,) = sqlx::query_as("SELECT count(*) FROM email_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 3);
    fresh.drop_db().await;
}
