//! The Library visibility and vault delete settings.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, Error, UpdateSettings, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping vault settings tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

struct Setup {
    pool: PgPool,
    owner: User,
    vault: Uuid,
    keyring: VaultKeyring,
}

async fn setup(mode: &str) -> Option<Setup> {
    let pool = pool_or_skip().await?;
    let email = format!("settings-{}@vaults.test", Uuid::new_v4());
    let owner = db::create_user(&pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user");
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Vault", "vault-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    set_mode(&pool, &owner, mode).await.expect("mode");
    Some(Setup {
        pool,
        owner,
        vault: vault.id,
        keyring,
    })
}

async fn set_mode(pool: &PgPool, owner: &User, mode: &str) -> linkdean_core::Result<()> {
    db::update_user_settings(
        pool,
        owner.id,
        &UpdateSettings {
            show_hidden: None,
            vault_search_mode: None,
            theme: None,
            vault_auto_lock_minutes: None,
            default_landing: None,
            library_view: None,
            trash_retention_days: None,
            show_deleted_in_library: None,
            show_vault_in_library: None,
            vault_delete_mode: Some(mode.into()),
        },
    )
    .await
    .map(|_| ())
}

async fn vault_bookmark(s: &Setup) -> Uuid {
    db::create_bookmark(
        &s.pool,
        s.owner.id,
        CreateBookmark {
            url: "https://vault.example/".into(),
            vault_id: Some(s.vault),
            ..Default::default()
        },
        &s.keyring,
    )
    .await
    .expect("bookmark")
    .id
}

async fn vault_category(s: &Setup, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        &s.pool,
        s.owner.id,
        CreateCategory {
            name: name.into(),
            parent_id,
            is_hidden: false,
            vault_id: Some(s.vault),
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn row_exists(pool: &PgPool, table: &str, id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(&format!(
        "SELECT EXISTS(SELECT 1 FROM {table} WHERE id = $1)"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("exists")
}

#[tokio::test]
async fn settings_default_and_reject_unknown_modes() {
    let Some(s) = setup("trash").await else {
        return;
    };
    let got = db::get_user_settings(&s.pool, s.owner.id)
        .await
        .expect("get");
    assert!(got.show_vault_in_library);
    assert_eq!(got.vault_delete_mode, "trash");
    let bad = set_mode(&s.pool, &s.owner, "shred").await;
    assert!(matches!(bad, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn permanent_mode_removes_vault_items() {
    let Some(s) = setup("permanent").await else {
        return;
    };
    let bookmark = vault_bookmark(&s).await;
    let parent = vault_category(&s, "Parent", None).await;
    let child = vault_category(&s, "Child", Some(parent)).await;

    db::delete_bookmark_by_mode(&s.pool, s.owner.id, bookmark)
        .await
        .expect("delete bookmark");
    db::delete_category_by_mode(&s.pool, s.owner.id, parent)
        .await
        .expect("delete category");

    assert!(!row_exists(&s.pool, "bookmarks", bookmark).await);
    assert!(!row_exists(&s.pool, "categories", parent).await);
    assert!(!row_exists(&s.pool, "categories", child).await);
}

#[tokio::test]
async fn trash_mode_keeps_vault_items_with_a_neutral_name() {
    let Some(s) = setup("trash").await else {
        return;
    };
    let bookmark = vault_bookmark(&s).await;
    db::delete_bookmark_by_mode(&s.pool, s.owner.id, bookmark)
        .await
        .expect("delete");

    assert!(row_exists(&s.pool, "bookmarks", bookmark).await);
    let trash = db::list_trash(&s.pool, s.owner.id).await.expect("trash");
    assert_eq!(
        trash
            .iter()
            .find(|t| t.id == bookmark)
            .expect("listed")
            .title,
        "Vault item"
    );
}

#[tokio::test]
async fn permanent_mode_leaves_plain_bookmarks_in_trash() {
    let Some(s) = setup("permanent").await else {
        return;
    };
    let plain = db::create_bookmark(
        &s.pool,
        s.owner.id,
        CreateBookmark {
            url: "https://plain.example/".into(),
            ..Default::default()
        },
        &s.keyring,
    )
    .await
    .expect("bookmark")
    .id;
    db::delete_bookmark_by_mode(&s.pool, s.owner.id, plain)
        .await
        .expect("delete");
    assert!(row_exists(&s.pool, "bookmarks", plain).await);
}

#[tokio::test]
async fn an_item_already_in_trash_stays_when_the_mode_changes() {
    let Some(s) = setup("trash").await else {
        return;
    };
    let bookmark = vault_bookmark(&s).await;
    db::delete_bookmark_by_mode(&s.pool, s.owner.id, bookmark)
        .await
        .expect("trash");
    set_mode(&s.pool, &s.owner, "permanent")
        .await
        .expect("mode");
    db::delete_bookmark_by_mode(&s.pool, s.owner.id, bookmark)
        .await
        .expect("delete again");
    assert!(row_exists(&s.pool, "bookmarks", bookmark).await);
}
