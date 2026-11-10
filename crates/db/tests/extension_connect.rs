//! Extension sign-in codes are single-use, short-lived and bound to their verifier and redirect.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::extension_connect::pkce_challenge;
use linkdean_core::User;
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping extension connect tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("ext-{}@extension.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const OTHER_VERIFIER: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn redirect() -> String {
    format!("https://{}.chromiumapp.org/", "a".repeat(32))
}

async fn code(pool: &PgPool, owner: &User) -> String {
    db::create_extension_code(
        pool,
        owner.id,
        &pkce_challenge(VERIFIER),
        &redirect(),
        "Browser extension · Chrome",
    )
    .await
    .expect("code")
}

#[tokio::test]
async fn a_code_works_once_with_its_verifier() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let c = code(&pool, &owner).await;
    let grant = db::take_extension_code(&pool, &c, VERIFIER, &redirect())
        .await
        .expect("take")
        .expect("grant");
    assert_eq!(grant.user_id, owner.id);
    assert_eq!(grant.key_name, "Browser extension · Chrome");
    assert!(db::take_extension_code(&pool, &c, VERIFIER, &redirect())
        .await
        .expect("again")
        .is_none());
}

#[tokio::test]
async fn a_wrong_verifier_or_redirect_burns_the_code() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let c = code(&pool, &owner).await;
    assert!(
        db::take_extension_code(&pool, &c, OTHER_VERIFIER, &redirect())
            .await
            .expect("wrong verifier")
            .is_none()
    );
    assert!(db::take_extension_code(&pool, &c, VERIFIER, &redirect())
        .await
        .expect("after burn")
        .is_none());

    let c = code(&pool, &owner).await;
    let other = format!("https://{}.chromiumapp.org/", "b".repeat(32));
    assert!(db::take_extension_code(&pool, &c, VERIFIER, &other)
        .await
        .expect("wrong redirect")
        .is_none());
}

#[tokio::test]
async fn an_expired_code_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let c = code(&pool, &owner).await;
    sqlx::query("UPDATE extension_connect_codes SET expires_at = now() - interval '1 second' WHERE user_id = $1")
        .bind(owner.id)
        .execute(&pool)
        .await
        .expect("expire");
    assert!(db::take_extension_code(&pool, &c, VERIFIER, &redirect())
        .await
        .expect("expired")
        .is_none());
}
