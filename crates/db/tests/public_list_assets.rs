//! Anonymous visitors of a public list can load its bookmarks' stored images, and nothing
//! else.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, Error, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping public list asset tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn bookmark(pool: &PgPool, owner: Uuid, category: Option<Uuid>, private: bool) -> Uuid {
    let id = db::create_bookmark(
        pool,
        owner,
        CreateBookmark {
            url: format!("https://assets.example/{}/", Uuid::new_v4()),
            title: Some("Asset BM".into()),
            category_id: category,
            category_ids: category.into_iter().collect(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id;
    // What the worker stores once it has cached the images.
    sqlx::query(
        "UPDATE bookmarks SET favicon_url = $2, preview_image_url = $3, has_preview = TRUE, \
         is_private = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(format!("/api/bookmarks/{id}/favicon"))
    .bind(format!("/api/bookmarks/{id}/og"))
    .bind(private)
    .execute(pool)
    .await
    .expect("set assets");
    id
}

#[tokio::test]
async fn public_list_images_are_reachable_only_through_the_list() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("assets-{}@public.test", Uuid::new_v4());
    let owner = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    let list = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Shared reading".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id;
    let listed = bookmark(&pool, owner.id, Some(list), false).await;
    let private = bookmark(&pool, owner.id, Some(list), true).await;
    let elsewhere = bookmark(&pool, owner.id, None, false).await;

    let slug = db::enable_category_public_link(&pool, owner.id, list)
        .await
        .expect("enable")
        .slug
        .expect("slug");

    let view = db::get_public_list_by_slug(&pool, &slug)
        .await
        .expect("public list");
    assert_eq!(view.bookmarks.len(), 1);
    assert_eq!(view.total, 1);
    let b = &view.bookmarks[0];
    assert_eq!(b.id, listed);
    let base = format!("/api/public/lists/{slug}/bookmarks/{listed}");
    assert_eq!(
        b.favicon_url.as_deref(),
        Some(format!("{base}/favicon").as_str())
    );
    assert_eq!(
        b.preview_image_url.as_deref(),
        Some(format!("{base}/og").as_str())
    );
    assert!(b
        .preview_url
        .as_deref()
        .expect("preview url")
        .starts_with(&format!("{base}/preview?v=")));

    assert_eq!(
        db::get_public_list_bookmark(&pool, &slug, listed)
            .await
            .expect("listed bookmark")
            .id,
        listed
    );
    for (id, why) in [
        (private, "a private bookmark"),
        (elsewhere, "a bookmark outside the list"),
        (Uuid::new_v4(), "an unknown id"),
    ] {
        assert!(
            matches!(
                db::get_public_list_bookmark(&pool, &slug, id).await,
                Err(Error::NotFound)
            ),
            "{why} must not be served"
        );
    }
    assert!(matches!(
        db::get_public_list_bookmark(&pool, "ldl_not-a-list", listed).await,
        Err(Error::NotFound)
    ));

    db::delete_bookmark(&pool, owner.id, listed)
        .await
        .expect("trash");
    assert!(
        matches!(
            db::get_public_list_bookmark(&pool, &slug, listed).await,
            Err(Error::NotFound)
        ),
        "a trashed bookmark must not be served"
    );

    db::disable_category_public_link(&pool, owner.id, list)
        .await
        .expect("disable");
    assert!(matches!(
        db::get_public_list_bookmark(&pool, &slug, private).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn a_banned_owners_public_list_goes_dark_until_the_ban_ends() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let mk = |tag: &str| format!("ban-{tag}-{}@public.test", Uuid::new_v4());
    let owner = db::create_user(&pool, &mk("owner"), Some("password-long"), None, None, None)
        .await
        .expect("owner");
    let moderator = db::create_user(&pool, &mk("mod"), Some("password-long"), None, None, None)
        .await
        .expect("moderator");
    let list = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Banned reading".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id;
    let listed = bookmark(&pool, owner.id, Some(list), false).await;
    let slug = db::enable_category_public_link(&pool, owner.id, list)
        .await
        .expect("enable")
        .slug
        .expect("slug");
    db::get_public_list_by_slug(&pool, &slug)
        .await
        .expect("served before the ban");

    // `ban_user` needs no admin flag on the acting user; leaving admins alone keeps this test
    // from racing the moderation tests.
    let ban = linkdean_core::CreateBan {
        reason: "spam".into(),
        ends_at: None,
        days: None,
    };
    db::ban_user(&pool, moderator.id, owner.id, &ban)
        .await
        .expect("ban");
    assert!(matches!(
        db::get_public_list_by_slug(&pool, &slug).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::get_public_list_bookmark(&pool, &slug, listed).await,
        Err(Error::NotFound)
    ));

    let lift = linkdean_core::LiftBans {
        user_ids: vec![owner.id],
    };
    db::lift_bans(&pool, moderator.id, &lift)
        .await
        .expect("lift");
    db::get_public_list_by_slug(&pool, &slug)
        .await
        .expect("served again once the ban is lifted");
}
