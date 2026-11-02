//! Purging a trashed folder never deletes live folders under it.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateCategory, Error, UpdateCategory, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping folder purge tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@purge.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn folder(pool: &PgPool, owner: &User, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: "Folder".into(),
            parent_id,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder")
    .id
}

async fn parent_of(pool: &PgPool, id: Uuid) -> Option<Option<Uuid>> {
    sqlx::query_scalar("SELECT parent_id FROM categories WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .expect("parent")
}

#[tokio::test]
async fn restoring_only_the_child_moves_it_out_before_a_purge() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let parent = folder(&pool, &owner, None).await;
    let child = folder(&pool, &owner, Some(parent)).await;
    db::delete_category(&pool, owner.id, parent)
        .await
        .expect("trash");
    db::restore_category(&pool, owner.id, child)
        .await
        .expect("restore child");
    assert_eq!(parent_of(&pool, child).await, Some(None));

    db::purge_category(&pool, owner.id, parent)
        .await
        .expect("purge");
    assert_eq!(parent_of(&pool, child).await, Some(None), "child survives");
}

#[tokio::test]
async fn purge_rescues_a_live_child_left_under_a_trashed_parent() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let parent = folder(&pool, &owner, None).await;
    let trashed_mid = folder(&pool, &owner, Some(parent)).await;
    let live = folder(&pool, &owner, Some(trashed_mid)).await;
    sqlx::query("UPDATE categories SET deleted_at = now() WHERE id = ANY($1)")
        .bind(vec![parent, trashed_mid])
        .execute(&pool)
        .await
        .expect("trash two");

    db::purge_category(&pool, owner.id, parent)
        .await
        .expect("purge");
    assert_eq!(
        parent_of(&pool, trashed_mid).await,
        None,
        "trashed child goes"
    );
    assert_eq!(
        parent_of(&pool, live).await,
        Some(None),
        "live grandchild stays"
    );
}

#[tokio::test]
async fn retention_purge_rescues_live_children() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let parent = folder(&pool, &owner, None).await;
    let live = folder(&pool, &owner, Some(parent)).await;
    sqlx::query("UPDATE categories SET deleted_at = now() - interval '400 days' WHERE id = $1")
        .bind(parent)
        .execute(&pool)
        .await
        .expect("expire parent");

    db::purge_expired_trash(&pool).await.expect("retention");
    assert_eq!(parent_of(&pool, parent).await, None);
    assert_eq!(parent_of(&pool, live).await, Some(None));
}

#[tokio::test]
async fn no_new_or_moved_folders_under_a_trashed_parent() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let trashed = folder(&pool, &owner, None).await;
    let other = folder(&pool, &owner, None).await;
    db::delete_category(&pool, owner.id, trashed)
        .await
        .expect("trash");

    let created = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Inside".into(),
            parent_id: Some(trashed),
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await;
    assert!(matches!(created, Err(Error::BadRequest(_))), "{created:?}");

    let moved = db::update_category(
        &pool,
        owner.id,
        other,
        UpdateCategory {
            parent_id: Some(Some(trashed)),
            ..Default::default()
        },
    )
    .await;
    assert!(matches!(moved, Err(Error::BadRequest(_))), "{moved:?}");
}

#[tokio::test]
async fn a_folder_cannot_move_into_or_out_of_a_vault_by_reparenting() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "vaultmove").await;
    let (vault, _, _) = db::create_vault(&pool, owner.id, "V", "vault-passphrase")
        .await
        .expect("vault");
    let plain = folder(&pool, &owner, None).await;
    let in_vault = db::create_category(
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
    .expect("vault folder")
    .id;

    for (moving, parent) in [(plain, in_vault), (in_vault, plain)] {
        let res = db::update_category(
            &pool,
            owner.id,
            moving,
            UpdateCategory {
                parent_id: Some(Some(parent)),
                ..Default::default()
            },
        )
        .await;
        assert!(
            matches!(res, Err(Error::BadRequest(_))),
            "{moving} under {parent}"
        );
    }
}
