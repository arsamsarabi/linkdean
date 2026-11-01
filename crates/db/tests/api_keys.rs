//! API key create / auth / revoke + platform/user gates.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateApiKey, Error, UpdateAdminUser};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping api_keys tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for api_keys tests");
    db::migrate(&pool)
        .await
        .expect("migrate for api_keys tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@apikey.test", Uuid::new_v4())
}

#[tokio::test]
async fn create_auth_revoke_api_key() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user = db::create_user(
        &pool,
        &unique_email("owner"),
        Some("password-long-enough"),
        Some("Key Owner"),
        None,
        None,
    )
    .await
    .expect("create user");

    let created = db::create_api_key(
        &pool,
        &user,
        CreateApiKey {
            name: "cli".into(),
        },
    )
    .await
    .expect("create key");
    assert!(created.token.starts_with("ldk_"));
    assert!(created.key.prefix.starts_with("ldk_"));

    let listed = db::list_api_keys(&pool, user.id).await.expect("list");
    assert!(listed.iter().any(|k| k.id == created.key.id));

    let via_key = db::user_from_token(&pool, &created.token)
        .await
        .expect("auth")
        .expect("user");
    assert_eq!(via_key.id, user.id);

    db::revoke_api_key(&pool, user.id, created.key.id)
        .await
        .expect("revoke");
    assert!(db::user_from_token(&pool, &created.token)
        .await
        .expect("auth after revoke")
        .is_none());
}

#[tokio::test]
async fn platform_and_user_gates_block_keys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user = db::create_user(
        &pool,
        &unique_email("gated"),
        Some("password-long-enough"),
        None,
        None,
        None,
    )
    .await
    .expect("create user");

    let created = db::create_api_key(
        &pool,
        &user,
        CreateApiKey {
            name: "tmp".into(),
        },
    )
    .await
    .expect("create");

    let _ = db::set_api_keys_enabled(&pool, false)
        .await
        .expect("platform off");
    assert!(db::user_from_token(&pool, &created.token)
        .await
        .expect("auth")
        .is_none());
    assert!(matches!(
        db::create_api_key(
            &pool,
            &user,
            CreateApiKey {
                name: "blocked".into(),
            },
        )
        .await,
        Err(Error::Forbidden)
    ));
    let _ = db::set_api_keys_enabled(&pool, true)
        .await
        .expect("platform on");

    let _ = db::update_admin_user(
        &pool,
        user.id,
        UpdateAdminUser {
            api_keys_enabled: Some(false),
        },
    )
    .await
    .expect("user off");
    assert!(db::user_from_token(&pool, &created.token)
        .await
        .expect("auth")
        .is_none());

    // Restore for other tests sharing DB
    let _ = db::update_admin_user(
        &pool,
        user.id,
        UpdateAdminUser {
            api_keys_enabled: Some(true),
        },
    )
    .await;
}
