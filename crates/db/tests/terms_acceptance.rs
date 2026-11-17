//! Accepting a new Terms of Service version.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::Error;
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

const TERMS_VERSION: Option<&str> = Some("2026-11-15");

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping terms acceptance tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

#[tokio::test]
async fn an_account_on_old_terms_accepts_the_current_version() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("terms-{}@terms.test", Uuid::new_v4());
    let user = db::create_user(&pool, &email, None, None, None, None)
        .await
        .expect("user");
    sqlx::query("UPDATE users SET terms_version = '1999-01-01' WHERE id = $1")
        .bind(user.id)
        .execute(&pool)
        .await
        .expect("old terms");
    let user = db::find_user_by_id(&pool, user.id)
        .await
        .expect("find")
        .expect("user");
    assert!(!user.terms_current(TERMS_VERSION));
    assert!(
        !db::to_public(&pool, &user, TERMS_VERSION)
            .await
            .expect("public")
            .terms_current
    );
    // A server without terms asks for none, and records nothing.
    assert!(user.terms_current(None));
    db::accept_terms(&pool, user.id, None, "1999-01-01")
        .await
        .expect("no terms");

    // A stale version from an old tab is refused.
    let err = db::accept_terms(&pool, user.id, TERMS_VERSION, "1999-01-01")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "{err}");

    db::accept_terms(&pool, user.id, TERMS_VERSION, "2026-11-15")
        .await
        .expect("accept");
    let user = db::find_user_by_id(&pool, user.id)
        .await
        .expect("find")
        .expect("user");
    assert!(user.terms_current(TERMS_VERSION));
    let accepted_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT terms_accepted_at FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .expect("accepted at");
    assert!(accepted_at.is_some());
}
