//! Instance-wide checks hold under races: one invite makes one account, only one account can
//! claim an empty instance as admin, and admins demoting each other leave one admin.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset. Each test runs in its own new
//! database, because "the first user" and closed registration are instance-wide.

use linkdean_core::{CreateRegistrationInvite, Error, UpdateAdminUser};
use linkdean_db::{self as db, SignupGate};
use sqlx::AssertSqlSafe;
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
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.name
        )))
        .execute(&mut admin)
        .await
        .expect("drop database");
    }
}

async fn fresh_db_or_skip() -> Option<FreshDb> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping instance race tests: DATABASE_URL unset");
        return None;
    };
    let name = format!("signup_{}", Uuid::new_v4().simple());
    let mut admin = PgConnection::connect(&url).await.expect("admin");
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
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

fn gate(invite_code: Option<&str>, bootstrap_token_ok: Option<bool>) -> SignupGate<'_> {
    SignupGate {
        enforce: true,
        invite_code,
        bootstrap_token_ok,
        terms_version: None,
    }
}

async fn sign_up(pool: &PgPool, email: &str, gate: SignupGate<'_>) -> Result<Uuid, Error> {
    db::create_user_gated(pool, email, Some("password-long"), None, None, None, gate)
        .await
        .map(|u| u.id)
}

/// Starts `n` sign-ups at once, the i-th as `{prefix}-{i}@race.test`.
async fn race(
    pool: &PgPool,
    prefix: &str,
    n: usize,
    invite: Option<String>,
) -> Vec<Result<Uuid, Error>> {
    let mut set = tokio::task::JoinSet::new();
    for i in 0..n {
        let (pool, invite, email) = (
            pool.clone(),
            invite.clone(),
            format!("{prefix}-{i}@race.test"),
        );
        set.spawn(async move { sign_up(&pool, &email, gate(invite.as_deref(), None)).await });
    }
    set.join_all().await
}

#[tokio::test]
async fn racing_first_sign_ups_make_one_admin() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = &fresh.pool;
    let results = race(pool, "first", 6, None).await;
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    let (admins,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(admins, 1);
    fresh.drop_db().await;
}

#[tokio::test]
async fn the_first_account_needs_the_bootstrap_token_when_one_is_set() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = &fresh.pool;
    let refused = sign_up(pool, "squatter@race.test", gate(None, Some(false))).await;
    assert!(matches!(refused, Err(Error::Forbidden)), "{refused:?}");
    assert_eq!(db::count_users(pool).await.unwrap(), 0);

    sign_up(pool, "owner@race.test", gate(None, Some(true)))
        .await
        .expect("owner");
    // Once an admin exists the token no longer matters; registration rules apply.
    sign_up(pool, "member@race.test", gate(None, Some(false)))
        .await
        .expect("member while registration is open");
    fresh.drop_db().await;
}

#[tokio::test]
async fn one_invite_makes_one_account_when_sign_ups_race() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = &fresh.pool;
    let admin = sign_up(pool, "admin@race.test", gate(None, None))
        .await
        .expect("admin");
    db::set_registration_open(pool, false).await.expect("close");
    let refused = sign_up(pool, "nobody@race.test", gate(None, None)).await;
    assert!(matches!(refused, Err(Error::Forbidden)), "{refused:?}");

    let invite = db::create_registration_invite(
        pool,
        admin,
        CreateRegistrationInvite {
            email: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite");
    let results = race(pool, "invitee", 6, Some(invite.code.clone())).await;
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        db::count_users(pool).await.unwrap(),
        2,
        "failed sign-ups leave no user"
    );

    let reused = sign_up(pool, "late@race.test", gate(Some(&invite.code), None)).await;
    assert!(
        matches!(&reused, Err(Error::BadRequest(m)) if m == "invite already used"),
        "{reused:?}"
    );
    fresh.drop_db().await;
}

fn set_admin(is_admin: bool) -> UpdateAdminUser {
    UpdateAdminUser {
        is_admin: Some(is_admin),
        api_keys_enabled: None,
    }
}

#[tokio::test]
async fn admins_demoting_each_other_at_once_leave_one_admin() {
    let Some(fresh) = fresh_db_or_skip().await else {
        return;
    };
    let pool = &fresh.pool;
    let a = sign_up(pool, "a@race.test", gate(None, None))
        .await
        .expect("a");
    let b = sign_up(pool, "b@race.test", gate(None, None))
        .await
        .expect("b");
    for _ in 0..10 {
        db::update_admin_user(pool, b, set_admin(true))
            .await
            .expect("promote");
        let (pa, pb) = (pool.clone(), pool.clone());
        let (ra, rb) = tokio::join!(
            tokio::spawn(async move { db::update_admin_user(&pa, a, set_admin(false)).await }),
            tokio::spawn(async move { db::update_admin_user(&pb, b, set_admin(false)).await }),
        );
        let ok = [ra.unwrap().is_ok(), rb.unwrap().is_ok()];
        assert_eq!(ok.iter().filter(|o| **o).count(), 1, "{ok:?}");
        let (admins,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(admins, 1);
        // Put A back as the admin for the next round.
        db::update_admin_user(pool, a, set_admin(true))
            .await
            .expect("restore a");
        db::update_admin_user(pool, b, set_admin(false))
            .await
            .expect("reset b");
    }
    fresh.drop_db().await;
}
