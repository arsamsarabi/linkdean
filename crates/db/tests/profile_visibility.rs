//! Who can see a profile and its bookmarks.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, Error, User};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping profile_visibility tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    db::create_user(
        pool,
        &format!("{prefix}-{}@visibility.test", Uuid::new_v4()),
        Some("password-long"),
        None,
        None,
        None,
    )
    .await
    .expect("user")
}

async fn category(pool: &PgPool, owner: &User) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Visibility-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn bookmark(pool: &PgPool, owner: &User, category_ids: Vec<Uuid>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://visibility.example/{}", Uuid::new_v4()),
            title: Some("Bookmark".into()),
            category_ids,
            ..Default::default()
        },
        None,
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn new_rows_default_to_invisible() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let bm = bookmark(&pool, &owner, vec![cat]).await;

    assert_eq!(owner.profile_visibility, "hidden");
    let (visibility,): (String,) =
        sqlx::query_as("SELECT visibility FROM categories WHERE id = $1")
            .bind(cat)
            .fetch_one(&pool)
            .await
            .expect("category row");
    assert_eq!(visibility, "private");
    let (is_private,): (bool,) = sqlx::query_as("SELECT is_private FROM bookmarks WHERE id = $1")
        .bind(bm)
        .fetch_one(&pool)
        .await
        .expect("bookmark row");
    assert!(!is_private);
}

#[tokio::test]
async fn vault_category_cannot_leave_private_at_db_level() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "vault").await;
    let cat = category(&pool, &owner).await;
    sqlx::query("UPDATE categories SET is_vault = TRUE WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await
        .expect("mark vault");
    let denied = sqlx::query("UPDATE categories SET visibility = 'members' WHERE id = $1")
        .bind(cat)
        .execute(&pool)
        .await;
    assert!(denied.is_err());
}

async fn set_profile(pool: &PgPool, user: &User, visibility: &str) {
    sqlx::query("UPDATE users SET profile_visibility = $2 WHERE id = $1")
        .bind(user.id)
        .bind(visibility)
        .execute(pool)
        .await
        .expect("set profile visibility");
}

async fn can_open(pool: &PgPool, viewer: &User, target: &User) -> bool {
    match db::get_profile(pool, viewer.id, target.id).await {
        Ok(_) => true,
        Err(Error::NotFound) => false,
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[tokio::test]
async fn profile_access_matrix() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let cases = [
        ("members", false, true),
        ("members", true, true),
        ("followers", false, false),
        ("followers", true, true),
        ("hidden", false, false),
        ("hidden", true, false),
    ];
    for (setting, follows, expected) in cases {
        let target = user(&pool, "target").await;
        let viewer = user(&pool, "viewer").await;
        set_profile(&pool, &target, setting).await;
        if follows {
            db::follow_user(&pool, viewer.id, target.id)
                .await
                .expect("follow");
        }
        assert_eq!(
            can_open(&pool, &viewer, &target).await,
            expected,
            "{setting}, follows: {follows}"
        );
        assert!(can_open(&pool, &target, &target).await, "self, {setting}");
    }
}

#[tokio::test]
async fn blocked_users_get_not_found_in_both_directions() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a").await;
    let b = user(&pool, "b").await;
    set_profile(&pool, &a, "members").await;
    set_profile(&pool, &b, "members").await;
    db::block_user(&pool, a.id, b.id).await.expect("block");
    assert!(!can_open(&pool, &a, &b).await);
    assert!(!can_open(&pool, &b, &a).await);
}

#[tokio::test]
async fn unknown_user_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let viewer = user(&pool, "viewer").await;
    let missing = db::get_profile(&pool, viewer.id, Uuid::new_v4()).await;
    assert!(matches!(missing, Err(Error::NotFound)));
}

#[tokio::test]
async fn email_and_setting_are_self_only() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;

    let own = db::get_profile(&pool, target.id, target.id)
        .await
        .expect("own");
    assert!(own.is_self);
    assert_eq!(own.email.as_deref(), Some(target.email.as_str()));
    assert_eq!(own.profile_visibility.as_deref(), Some("members"));

    let other = db::get_profile(&pool, viewer.id, target.id)
        .await
        .expect("other");
    assert!(!other.is_self);
    assert!(other.email.is_none());
    assert!(other.profile_visibility.is_none());
    assert!(!other.display_name.contains('@'));
    assert_eq!(other.display_name, "User");
}

#[tokio::test]
async fn losing_the_follow_loses_access_immediately() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "followers").await;
    db::follow_user(&pool, viewer.id, target.id)
        .await
        .expect("follow");
    assert!(can_open(&pool, &viewer, &target).await);
    db::unfollow_user(&pool, viewer.id, target.id)
        .await
        .expect("unfollow");
    assert!(!can_open(&pool, &viewer, &target).await);
}
