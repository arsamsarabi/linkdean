//! Several vaults per user, with their own keys.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{Error, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping vault tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@vaults.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user")
}

#[tokio::test]
async fn two_vaults_have_separate_keys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (work, work_key, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("work");
    let (home, home_key, _) = db::create_vault(&pool, owner.id, "Home", "home-passphrase")
        .await
        .expect("home");
    assert_ne!(work_key, home_key);

    let unlocked = db::unlock_vault(&pool, owner.id, work.id, "work-passphrase")
        .await
        .expect("unlock");
    assert_eq!(unlocked, work_key);
    let wrong = db::unlock_vault(&pool, owner.id, home.id, "work-passphrase").await;
    assert!(matches!(wrong, Err(Error::VaultUnlockFailed)));
}

#[tokio::test]
async fn names_are_unique_per_user_ignoring_case() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let other = user(&pool, "other").await;
    db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("first");
    let dup = db::create_vault(&pool, owner.id, "work", "work-passphrase").await;
    assert!(matches!(dup, Err(Error::Conflict(_))));
    db::create_vault(&pool, other.id, "Work", "work-passphrase")
        .await
        .expect("another user");
}

#[tokio::test]
async fn another_users_vault_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let stranger = user(&pool, "stranger").await;
    let (vault, _, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");

    let got = db::unlock_vault(&pool, stranger.id, vault.id, "work-passphrase").await;
    assert!(matches!(got, Err(Error::NotFound)));
    let got = db::rename_vault(&pool, stranger.id, vault.id, "Mine").await;
    assert!(matches!(got, Err(Error::NotFound)));
}

#[tokio::test]
async fn recovery_key_sets_a_new_passphrase() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, recovery) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");

    let wrong = db::recover_vault(&pool, owner.id, vault.id, "NOPE", "new-passphrase").await;
    assert!(matches!(wrong, Err(Error::VaultUnlockFailed)));

    let messy = recovery.to_ascii_lowercase().replace('-', " ");
    let got = db::recover_vault(&pool, owner.id, vault.id, &messy, "new-passphrase")
        .await
        .expect("recover");
    assert_eq!(got, key);

    assert_eq!(
        db::unlock_vault(&pool, owner.id, vault.id, "new-passphrase")
            .await
            .expect("new"),
        key
    );
    let old = db::unlock_vault(&pool, owner.id, vault.id, "work-passphrase").await;
    assert!(matches!(old, Err(Error::VaultUnlockFailed)));
}

#[tokio::test]
async fn replacing_a_recovery_key_invalidates_the_old_one() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, old) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");

    let new = db::replace_recovery_key(&pool, owner.id, vault.id, &key)
        .await
        .expect("replace");
    let got = db::recover_vault(&pool, owner.id, vault.id, &old, "new-passphrase").await;
    assert!(matches!(got, Err(Error::VaultUnlockFailed)));
    db::recover_vault(&pool, owner.id, vault.id, &new, "new-passphrase")
        .await
        .expect("recover");
}
