//! Password reset tokens: single use, expiry, newest wins, and what a reset ends.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::Error;
use linkdean_db::{self as db, ResetOutcome};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping password reset tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &sqlx::PgPool) -> (Uuid, String) {
    let email = format!("reset-{}@reset.test", Uuid::new_v4());
    let u = db::create_user(pool, &email, Some("old-password-1"), None, None, None)
        .await
        .unwrap();
    (u.id, u.email)
}

async fn live_tokens(pool: &sqlx::PgPool, uid: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM password_reset_tokens WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(uid)
    .fetch_one(pool)
    .await
    .unwrap();
    n
}

#[tokio::test]
async fn a_token_resets_once_and_ends_every_session() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, email) = user(&pool).await;
    db::create_session(&pool, uid, 30).await.unwrap();
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();

    let out = db::consume_reset_token(&pool, &token, "new-password-123", false)
        .await
        .unwrap();
    assert_eq!(
        out,
        ResetOutcome {
            user_id: uid,
            signed_out_sessions: 1,
            revoked_api_keys: 0
        }
    );
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM sessions WHERE user_id = $1")
        .bind(uid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    assert!(db::authenticate_local(&pool, &email, "new-password-123")
        .await
        .is_ok());
    assert!(db::authenticate_local(&pool, &email, "old-password-1")
        .await
        .is_err());

    let again = db::consume_reset_token(&pool, &token, "another-password-1", false).await;
    assert!(matches!(again, Err(Error::BadRequest(_))), "{again:?}");
}

#[tokio::test]
async fn a_reset_can_revoke_api_keys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    sqlx::query(
        "INSERT INTO api_keys (user_id, name, token_hash, prefix) VALUES ($1, 'reset test', $2, 'ld_test')",
    )
    .bind(uid)
    .bind(Uuid::new_v4().to_string())
    .execute(&pool)
    .await
    .unwrap();
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    let out = db::consume_reset_token(&pool, &token, "new-password-123", true)
        .await
        .unwrap();
    assert_eq!(out.revoked_api_keys, 1);
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM api_keys WHERE user_id = $1 AND revoked_at IS NULL")
            .bind(uid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn an_unknown_token_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let r = db::consume_reset_token(&pool, "not-a-real-token", "new-password-123", false).await;
    assert!(matches!(r, Err(Error::BadRequest(_))), "{r:?}");
}

#[tokio::test]
async fn an_expired_token_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE password_reset_tokens SET expires_at = now() - interval '1 second' WHERE user_id = $1",
    )
    .bind(uid)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        db::consume_reset_token(&pool, &token, "new-password-123", false).await,
        Err(Error::BadRequest(_))
    ));
}

#[tokio::test]
async fn a_new_token_voids_the_old_one() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    let old = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    let new = db::create_reset_token(&pool, uid, chrono::Duration::hours(24), Some(uid))
        .await
        .unwrap();
    assert_eq!(live_tokens(&pool, uid).await, 1);
    assert!(
        db::consume_reset_token(&pool, &old, "new-password-123", false)
            .await
            .is_err()
    );
    assert!(
        db::consume_reset_token(&pool, &new, "new-password-123", false)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn changing_the_password_voids_a_live_token() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    db::change_password(
        &pool,
        uid,
        "old-password-1",
        "changed-password-1",
        None,
        false,
    )
    .await
    .unwrap();
    assert_eq!(live_tokens(&pool, uid).await, 0);
    assert!(
        db::consume_reset_token(&pool, &token, "new-password-123", false)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn an_account_without_a_password_cannot_reset() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET password_hash = NULL WHERE id = $1")
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        db::consume_reset_token(&pool, &token, "new-password-123", false).await,
        Err(Error::BadRequest(_))
    ));
}

#[tokio::test]
async fn two_concurrent_uses_only_one_wins() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        db::consume_reset_token(&pool, &token, "first-password-1", false),
        db::consume_reset_token(&pool, &token, "second-password-1", false),
    );
    assert_eq!([a.is_ok(), b.is_ok()].iter().filter(|ok| **ok).count(), 1);
}

#[tokio::test]
async fn a_short_password_is_refused_and_the_token_survives() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    let token = db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    assert!(matches!(
        db::consume_reset_token(&pool, &token, "short", false).await,
        Err(Error::BadRequest(_))
    ));
    assert!(
        db::consume_reset_token(&pool, &token, "long-enough-password", false)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn old_tokens_are_purged() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (uid, _) = user(&pool).await;
    db::create_reset_token(&pool, uid, chrono::Duration::minutes(30), None)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE password_reset_tokens SET expires_at = now() - interval '2 days' WHERE user_id = $1",
    )
    .bind(uid)
    .execute(&pool)
    .await
    .unwrap();
    assert!(db::purge_old_reset_tokens(&pool).await.unwrap() >= 1);
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM password_reset_tokens WHERE user_id = $1")
            .bind(uid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn admin_summaries_say_whether_an_account_has_a_password() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (with_pw, _) = user(&pool).await;
    let (without_pw, _) = user(&pool).await;
    sqlx::query("UPDATE users SET password_hash = NULL WHERE id = $1")
        .bind(without_pw)
        .execute(&pool)
        .await
        .unwrap();
    let all = db::list_admin_users(&pool).await.unwrap();
    let find = |id| all.iter().find(|u| u.id == id).unwrap().has_password;
    assert!(find(with_pw));
    assert!(!find(without_pw));
}
