//! The optional global passphrase. It adds a way in and replaces nothing.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{Error, User, Vault};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping global passphrase tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("global-{}@vaults.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user")
}

async fn vault(pool: &PgPool, owner: &User, name: &str) -> (Vault, [u8; 32]) {
    let (vault, key, _) = db::create_vault(pool, owner.id, name, "vault-passphrase")
        .await
        .expect("vault");
    (vault, key)
}

#[tokio::test]
async fn unlocks_only_joined_vaults_and_own_passphrases_keep_working() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, joined_key) = vault(&pool, &owner, "Joined").await;
    let (apart, _) = vault(&pool, &owner, "Apart").await;

    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, joined.id, &joined_key)
        .await
        .expect("join");

    let opened = db::unlock_with_global(&pool, owner.id, "global-passphrase")
        .await
        .expect("unlock");
    assert_eq!(opened, vec![(joined.id, joined_key)]);
    assert!(opened.iter().all(|(id, _)| *id != apart.id));

    assert_eq!(
        db::unlock_vault(&pool, owner.id, joined.id, "vault-passphrase")
            .await
            .expect("own passphrase"),
        joined_key
    );
}

#[tokio::test]
async fn wrong_passphrase_fails() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key) = vault(&pool, &owner, "Joined").await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");

    db::join_global(&pool, owner.id, joined.id, &key)
        .await
        .expect("join");
    let open = db::unlock_with_global(&pool, owner.id, "wrong-passphrase").await;
    assert!(matches!(open, Err(Error::VaultUnlockFailed)));
}

#[tokio::test]
async fn enabling_twice_conflicts() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    let again = db::enable_global_passphrase(&pool, owner.id, "other-passphrase").await;
    assert!(matches!(again, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn changing_keeps_vaults_joined() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key) = vault(&pool, &owner, "Joined").await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, joined.id, &key)
        .await
        .expect("join");

    db::change_global_passphrase(&pool, owner.id, "global-passphrase", "changed-passphrase")
        .await
        .expect("change");

    let old = db::unlock_with_global(&pool, owner.id, "global-passphrase").await;
    assert!(matches!(old, Err(Error::VaultUnlockFailed)));
    let opened = db::unlock_with_global(&pool, owner.id, "changed-passphrase")
        .await
        .expect("unlock");
    assert_eq!(opened, vec![(joined.id, key)]);
}

#[tokio::test]
async fn leaving_and_disabling_clear_the_wrapped_keys() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (first, first_key) = vault(&pool, &owner, "First").await;
    let (second, second_key) = vault(&pool, &owner, "Second").await;
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, first.id, &first_key)
        .await
        .expect("join first");
    db::join_global(&pool, owner.id, second.id, &second_key)
        .await
        .expect("join second");

    db::leave_global(&pool, owner.id, first.id)
        .await
        .expect("leave");
    let opened = db::unlock_with_global(&pool, owner.id, "global-passphrase")
        .await
        .expect("unlock");
    assert_eq!(opened, vec![(second.id, second_key)]);

    assert!(
        db::disable_global_passphrase(&pool, owner.id, "wrong-passphrase")
            .await
            .is_err(),
        "a wrong passphrase must not turn the global passphrase off"
    );
    assert!(db::has_global_passphrase(&pool, owner.id)
        .await
        .expect("still on"));
    db::disable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("disable");
    assert!(!db::has_global_passphrase(&pool, owner.id)
        .await
        .expect("state"));
    assert!(db::list_vaults(&pool, owner.id)
        .await
        .expect("list")
        .iter()
        .all(|v| v.global_wrapped_key.is_none()));
    db::unlock_vault(&pool, owner.id, second.id, "vault-passphrase")
        .await
        .expect("own passphrase survives");
}

#[tokio::test]
async fn recovery_does_not_change_membership() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key, recovery) = db::create_vault(&pool, owner.id, "Joined", "vault-passphrase")
        .await
        .expect("vault");
    db::enable_global_passphrase(&pool, owner.id, "global-passphrase")
        .await
        .expect("enable");
    db::join_global(&pool, owner.id, joined.id, &key)
        .await
        .expect("join");

    db::recover_vault(&pool, owner.id, joined.id, &recovery, "new-passphrase")
        .await
        .expect("recover");
    assert_eq!(
        db::unlock_with_global(&pool, owner.id, "global-passphrase")
            .await
            .expect("unlock"),
        vec![(joined.id, key)]
    );
}

#[tokio::test]
async fn joining_needs_the_global_passphrase_turned_on() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let (joined, key) = vault(&pool, &owner, "Joined").await;

    let got = db::join_global(&pool, owner.id, joined.id, &key).await;
    assert!(matches!(got, Err(Error::BadRequest(_))));
}
