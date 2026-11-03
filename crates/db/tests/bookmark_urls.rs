//! Only http(s) URLs are stored, on create and on update.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, Error, UpdateBookmark, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping bookmark URL tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

#[test]
fn only_http_and_https_are_web_urls() {
    for ok in [
        "https://a.example",
        "http://a.example/x",
        "HTTPS://A.EXAMPLE",
        "  https://a.example",
    ] {
        assert!(db::require_web_url(ok).is_ok(), "{ok}");
    }
    for bad in [
        "javascript:alert(1)",
        " JavaScript:alert(1)",
        "java\tscript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///etc/passwd",
        "a.example",
        "",
    ] {
        assert!(
            matches!(db::require_web_url(bad), Err(Error::BadRequest(_))),
            "{bad:?}"
        );
    }
}

#[tokio::test]
async fn create_and_update_refuse_other_schemes() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("urls-{}@urls.test", Uuid::new_v4());
    let owner = db::create_user(&pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user");
    let keyring = VaultKeyring::default();

    let bad = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "javascript:alert(1)".into(),
            ..Default::default()
        },
        &keyring,
    )
    .await;
    assert!(matches!(bad, Err(Error::BadRequest(_))), "{bad:?}");

    let good = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://ok.example/".into(),
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("bookmark");
    let changed = db::update_bookmark(
        &pool,
        owner.id,
        good.id,
        UpdateBookmark {
            url: Some("data:text/html,hi".into()),
            ..Default::default()
        },
        &keyring,
    )
    .await;
    assert!(matches!(changed, Err(Error::BadRequest(_))), "{changed:?}");
    let (url,): (String,) = sqlx::query_as("SELECT url FROM bookmarks WHERE id = $1")
        .bind(good.id)
        .fetch_one(&pool)
        .await
        .expect("url");
    assert_eq!(url, "https://ok.example/");
}
