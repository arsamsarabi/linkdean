//! A list can never end up inside itself.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateCategory, Error, UpdateCategory, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping category cycle tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("cycles-{}@limits.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn category(pool: &PgPool, owner: &User, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Cycle-{}", Uuid::new_v4()),
            parent_id,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

fn move_under(parent: Option<Uuid>) -> UpdateCategory {
    UpdateCategory {
        name: None,
        parent_id: Some(parent),
        is_hidden: None,
        position: None,
        icon: None,
        visibility: None,
    }
}

#[tokio::test]
async fn a_list_cannot_move_under_itself_or_a_descendant() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let top = category(&pool, &owner, None).await;
    let child = category(&pool, &owner, Some(top)).await;
    let grandchild = category(&pool, &owner, Some(child)).await;

    for parent in [top, child, grandchild] {
        let err = db::update_category(&pool, owner.id, top, move_under(Some(parent)))
            .await
            .expect_err("cycle");
        assert!(matches!(err, Error::BadRequest(_)), "got {err:?}");
    }
}

#[tokio::test]
async fn ordinary_moves_still_work() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let top = category(&pool, &owner, None).await;
    let other = category(&pool, &owner, None).await;
    let child = category(&pool, &owner, Some(top)).await;

    let moved = db::update_category(&pool, owner.id, child, move_under(Some(other)))
        .await
        .expect("move");
    assert_eq!(moved.parent_id, Some(other));

    let moved = db::update_category(&pool, owner.id, child, move_under(None))
        .await
        .expect("move to root");
    assert_eq!(moved.parent_id, None);
}

#[tokio::test]
async fn concurrent_opposite_moves_cannot_make_a_cycle() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    for _ in 0..10 {
        let owner = user(&pool).await;
        let a = category(&pool, &owner, None).await;
        let b = category(&pool, &owner, None).await;

        let (first, second) = tokio::join!(
            db::update_category(&pool, owner.id, a, move_under(Some(b))),
            db::update_category(&pool, owner.id, b, move_under(Some(a))),
        );
        assert!(
            first.is_err() || second.is_err(),
            "both moves succeeded, so a and b are each other's parent"
        );
    }
}

#[tokio::test]
async fn the_pool_limits_statement_time() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let (timeout,): (String,) = sqlx::query_as("SHOW statement_timeout")
        .fetch_one(&pool)
        .await
        .expect("show");
    assert_eq!(timeout, "30s");
}
