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

async fn set_category(pool: &PgPool, id: Uuid, visibility: &str) {
    sqlx::query("UPDATE categories SET visibility = $2 WHERE id = $1")
        .bind(id)
        .bind(visibility)
        .execute(pool)
        .await
        .expect("set category visibility");
}

async fn visible_ids(pool: &PgPool, viewer: &User, target: &User) -> Vec<Uuid> {
    db::profile_visible_bookmarks(pool, viewer.id, target.id, None, None)
        .await
        .expect("visible bookmarks")
        .bookmarks
        .into_iter()
        .map(|b| b.id)
        .collect()
}

#[tokio::test]
async fn category_tier_matrix() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let cases = [
        ("private", false, false),
        ("private", true, false),
        ("followers", false, false),
        ("followers", true, true),
        ("members", false, true),
        ("members", true, true),
    ];
    for (tier, follows, expected) in cases {
        let target = user(&pool, "target").await;
        let viewer = user(&pool, "viewer").await;
        set_profile(&pool, &target, "members").await;
        let cat = category(&pool, &target).await;
        set_category(&pool, cat, tier).await;
        let bm = bookmark(&pool, &target, vec![cat]).await;
        if follows {
            db::follow_user(&pool, viewer.id, target.id)
                .await
                .expect("follow");
        }
        let ids = visible_ids(&pool, &viewer, &target).await;
        assert_eq!(ids.contains(&bm), expected, "{tier}, follows: {follows}");
    }
}

#[tokio::test]
async fn excluded_bookmarks_stay_excluded() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;
    let cat = category(&pool, &target).await;
    set_category(&pool, cat, "members").await;

    let shown = bookmark(&pool, &target, vec![cat]).await;
    let private = bookmark(&pool, &target, vec![cat]).await;
    sqlx::query("UPDATE bookmarks SET is_private = TRUE WHERE id = $1")
        .bind(private)
        .execute(&pool)
        .await
        .expect("mark private");
    let vault = bookmark(&pool, &target, vec![cat]).await;
    sqlx::query("UPDATE bookmarks SET is_vault = TRUE WHERE id = $1")
        .bind(vault)
        .execute(&pool)
        .await
        .expect("mark vault");
    let trashed = bookmark(&pool, &target, vec![cat]).await;
    db::delete_bookmark(&pool, target.id, trashed)
        .await
        .expect("trash");
    let uncategorized = bookmark(&pool, &target, vec![]).await;

    let ids = visible_ids(&pool, &viewer, &target).await;
    assert_eq!(
        ids,
        vec![shown],
        "uncategorized {uncategorized} must not show"
    );
}

#[tokio::test]
async fn any_allowing_category_is_enough() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;
    let open = category(&pool, &target).await;
    let closed = category(&pool, &target).await;
    set_category(&pool, open, "members").await;
    let bm = bookmark(&pool, &target, vec![closed, open]).await;
    assert_eq!(visible_ids(&pool, &viewer, &target).await, vec![bm]);
}

#[tokio::test]
async fn a_third_partys_category_does_not_publish_a_bookmark() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let third = user(&pool, "third").await;
    let viewer = user(&pool, "viewer").await;
    set_profile(&pool, &target, "members").await;
    let theirs = category(&pool, &third).await;
    set_category(&pool, theirs, "members").await;
    let bm = bookmark(&pool, &target, vec![]).await;
    sqlx::query("INSERT INTO bookmark_categories (bookmark_id, category_id) VALUES ($1, $2)")
        .bind(bm)
        .bind(theirs)
        .execute(&pool)
        .await
        .expect("link");
    assert!(visible_ids(&pool, &viewer, &target).await.is_empty());
}

#[tokio::test]
async fn hidden_profile_hides_bookmarks_and_self_sees_preview() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let viewer = user(&pool, "viewer").await;
    let cat = category(&pool, &target).await;
    set_category(&pool, cat, "followers").await;
    let bm = bookmark(&pool, &target, vec![cat]).await;

    let denied = db::profile_visible_bookmarks(&pool, viewer.id, target.id, None, None).await;
    assert!(matches!(denied, Err(Error::NotFound)));
    assert_eq!(visible_ids(&pool, &target, &target).await, vec![bm]);
}

#[tokio::test]
async fn paging_reports_the_total() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let target = user(&pool, "target").await;
    let cat = category(&pool, &target).await;
    set_category(&pool, cat, "members").await;
    for _ in 0..3 {
        bookmark(&pool, &target, vec![cat]).await;
    }
    let page = db::profile_visible_bookmarks(&pool, target.id, target.id, Some(2), Some(2))
        .await
        .expect("page");
    assert_eq!(page.total, 3);
    assert_eq!(page.bookmarks.len(), 1);

    for (limit, offset) in [(Some(0), None), (Some(101), None), (None, Some(-1))] {
        let bad = db::profile_visible_bookmarks(&pool, target.id, target.id, limit, offset).await;
        assert!(matches!(bad, Err(Error::BadRequest(_))));
    }
}
