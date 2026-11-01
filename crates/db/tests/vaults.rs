//! Several vaults per user, with their own keys.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, Error, User, VaultKeyring};
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

#[tokio::test]
async fn deleting_a_vault_removes_its_contents() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    let cat = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Secret".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: Some(vault.id),
            icon: None,
        },
    )
    .await
    .expect("category");
    let bookmark = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://secret.example/".into(),
            category_ids: vec![cat.id],
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("bookmark");

    let wrong = db::delete_vault(&pool, &owner, vault.id, "Nope", Some("account-password")).await;
    assert!(matches!(wrong, Err(Error::BadRequest(_))));
    let wrong = db::delete_vault(&pool, &owner, vault.id, "Work", Some("bad")).await;
    assert!(matches!(wrong, Err(Error::BadRequest(_))));

    db::delete_vault(&pool, &owner, vault.id, "Work", Some("account-password"))
        .await
        .expect("delete");
    assert!(db::get_bookmark_raw(&pool, owner.id, bookmark.id)
        .await
        .is_err());
    assert!(db::list_vaults(&pool, owner.id)
        .await
        .expect("list")
        .is_empty());
}

async fn vault_category(pool: &PgPool, owner: &User, vault_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Cat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

#[tokio::test]
async fn a_locked_vaults_items_stay_hidden_while_another_is_open() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (open, open_key, _) = db::create_vault(&pool, owner.id, "Open", "open-passphrase")
        .await
        .expect("open");
    let (shut, shut_key, _) = db::create_vault(&pool, owner.id, "Shut", "shut-passphrase")
        .await
        .expect("shut");
    let mut both = VaultKeyring::default();
    both.insert(open.id, open_key);
    both.insert(shut.id, shut_key);

    let open_cat = vault_category(&pool, &owner, Some(open.id)).await;
    let shut_cat = vault_category(&pool, &owner, Some(shut.id)).await;
    for (cat, url) in [
        (open_cat, "https://open.example/"),
        (shut_cat, "https://shut.example/"),
    ] {
        db::create_bookmark(
            &pool,
            owner.id,
            CreateBookmark {
                url: url.into(),
                category_ids: vec![cat],
                ..Default::default()
            },
            &both,
        )
        .await
        .expect("bookmark");
    }

    let mut only_open = VaultKeyring::default();
    only_open.insert(open.id, open_key);
    let list = db::list_bookmarks(&pool, owner.id, None, true, &only_open, false)
        .await
        .expect("list");
    assert_eq!(
        list.iter().map(|b| b.url.as_str()).collect::<Vec<_>>(),
        vec!["https://open.example/"]
    );

    let cats = db::list_categories(&pool, owner.id, false, &only_open.vault_ids(), false)
        .await
        .expect("categories");
    assert!(cats.iter().any(|c| c.id == open_cat));
    assert!(cats.iter().all(|c| c.id != shut_cat));
}

#[tokio::test]
async fn a_loose_bookmark_lives_in_a_vault_without_a_category() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Loose", "loose-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);

    let view = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://loose.example/".into(),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("bookmark");
    assert!(view.is_vault);
    assert!(view.categories.is_empty());
    assert_eq!(view.url, "https://loose.example/");

    let locked = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://loose.example/2".into(),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(locked, Err(Error::VaultLocked)));
}

#[tokio::test]
async fn a_bookmark_cannot_span_vaults() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (first, first_key, _) = db::create_vault(&pool, owner.id, "First", "first-passphrase")
        .await
        .expect("first");
    let (second, second_key, _) = db::create_vault(&pool, owner.id, "Second", "second-passphrase")
        .await
        .expect("second");
    let mut keyring = VaultKeyring::default();
    keyring.insert(first.id, first_key);
    keyring.insert(second.id, second_key);
    let first_cat = vault_category(&pool, &owner, Some(first.id)).await;
    let second_cat = vault_category(&pool, &owner, Some(second.id)).await;
    let plain_cat = vault_category(&pool, &owner, None).await;

    for category_ids in [vec![first_cat, second_cat], vec![first_cat, plain_cat]] {
        let got = db::create_bookmark(
            &pool,
            owner.id,
            CreateBookmark {
                url: "https://mixed.example/".into(),
                category_ids,
                ..Default::default()
            },
            &keyring,
        )
        .await;
        assert!(matches!(got, Err(Error::BadRequest(_))));
    }

    let wrong = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://mixed.example/".into(),
            category_ids: vec![first_cat],
            vault_id: Some(second.id),
            ..Default::default()
        },
        &keyring,
    )
    .await;
    assert!(matches!(wrong, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn a_subcategory_stays_in_its_parents_vault() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let (vault, _, _) = db::create_vault(&pool, owner.id, "Nest", "nest-passphrase")
        .await
        .expect("vault");
    let parent = vault_category(&pool, &owner, Some(vault.id)).await;

    let got = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Plain child".into(),
            parent_id: Some(parent),
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await;
    assert!(matches!(got, Err(Error::BadRequest(_))));
}
