//! Local sign-in: unknown emails fail like wrong passwords, and old hashes are upgraded.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::Error;
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping login tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

#[tokio::test]
async fn an_unknown_email_fails_like_a_wrong_password() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("nobody-{}@login.test", Uuid::new_v4());
    let err = db::authenticate_local(&pool, &email, "whatever-password")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unauthorized));
}

#[tokio::test]
async fn an_old_hash_is_replaced_at_sign_in() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    use argon2::password_hash::{PasswordHasher, SaltString};
    let email = format!("old-{}@login.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    let salt = SaltString::generate(&mut rand::thread_rng());
    let old = argon2::Argon2::default()
        .hash_password(b"password-long", &salt)
        .unwrap()
        .to_string();
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(user.id)
        .bind(&old)
        .execute(&pool)
        .await
        .expect("set old hash");

    db::authenticate_local(&pool, &email, "password-long")
        .await
        .expect("sign in");
    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("hash");
    assert_ne!(stored, old);
    assert!(!linkdean_core::crypto::password_needs_rehash(&stored));
    db::authenticate_local(&pool, &email, "password-long")
        .await
        .expect("sign in with new hash");
}
