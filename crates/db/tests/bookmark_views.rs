//! Lists build each bookmark's view in a batch. The batch must give the owner and an invitee
//! the same views as building one bookmark at a time.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    BookmarkView, CreateBookmark, CreateCategory, CreateCategoryShare, ShareAccess, UpdateBookmark,
    User, VaultKeyring,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping bookmark view tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool, prefix: &str, name: &str) -> User {
    let email = format!("{prefix}-{}@views.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    sqlx::query("UPDATE users SET display_name = $2 WHERE id = $1")
        .bind(user.id)
        .bind(name)
        .execute(pool)
        .await
        .expect("name");
    user
}

async fn folder(pool: &PgPool, owner: &User, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: name.into(),
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

async fn bookmark(pool: &PgPool, owner: &User, folders: &[Uuid], tags: &[&str]) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://views.example/{}/", Uuid::new_v4()),
            title: Some("View BM".into()),
            category_id: folders.first().copied(),
            category_ids: folders.to_vec(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn one_by_one(pool: &PgPool, viewer: Uuid, ids: &[Uuid]) -> Vec<BookmarkView> {
    let mut out = Vec::new();
    for id in ids {
        let raw = db::get_bookmark_raw(pool, viewer, *id).await.expect("raw");
        out.push(
            db::bookmark_to_view(pool, raw, &VaultKeyring::default(), viewer)
                .await
                .expect("view")
                .expect("visible"),
        );
    }
    out
}

fn json(views: &[&BookmarkView]) -> serde_json::Value {
    serde_json::to_value(views).expect("json")
}

#[tokio::test]
async fn listed_views_match_single_views_for_owner_and_invitee() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = person(&pool, "owner", "Owner Person").await;
    let invitee = person(&pool, "invitee", "Invitee Person").await;

    let root = folder(&pool, &owner, "Root", None).await;
    let child = folder(&pool, &owner, "Child", Some(root)).await;
    let secret = folder(&pool, &owner, "Secret", None).await;

    let in_child_and_secret = bookmark(&pool, &owner, &[child, secret], &["beta", "alpha"]).await;
    let in_root = bookmark(&pool, &owner, &[root], &[]).await;
    let unsorted = bookmark(&pool, &owner, &[], &["solo"]).await;

    db::update_bookmark(
        &pool,
        owner.id,
        in_child_and_secret,
        UpdateBookmark {
            is_favorite: Some(true),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("favourite");

    db::create_share(
        &pool,
        owner.id,
        root,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("share");
    for invite in db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox")
    {
        db::accept_share_invite(&pool, invitee.id, invite.id)
            .await
            .expect("accept");
    }
    db::update_bookmark(
        &pool,
        invitee.id,
        in_root,
        UpdateBookmark {
            is_unread: Some(true),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("invitee flag");

    let none = VaultKeyring::default();

    // The owner sees everything, with full paths and who it is shared with.
    let listed = db::list_bookmarks(&pool, owner.id, None, false, &none, false)
        .await
        .expect("owner list");
    let mine: Vec<&BookmarkView> = listed
        .iter()
        .filter(|v| [in_child_and_secret, in_root, unsorted].contains(&v.id))
        .collect();
    assert_eq!(mine.len(), 3);
    let ids: Vec<Uuid> = mine.iter().map(|v| v.id).collect();
    let single = one_by_one(&pool, owner.id, &ids).await;
    assert_eq!(json(&mine), json(&single.iter().collect::<Vec<_>>()));

    let both = mine.iter().find(|v| v.id == in_child_and_secret).unwrap();
    assert_eq!(both.tags, vec!["alpha", "beta"]);
    assert!(both.is_favorite);
    let mut paths: Vec<&str> = both.categories.iter().map(|c| c.path.as_str()).collect();
    paths.sort();
    assert_eq!(paths, vec!["Root / Child", "Secret"]);
    assert_eq!(both.shared_with, vec!["Invitee Person"]);
    assert!(both.shared_out);
    let alone = mine.iter().find(|v| v.id == unsorted).unwrap();
    assert!(alone.categories.is_empty());
    assert!(!alone.shared_out);

    // The invitee sees only the shared tree, their own flags, and the owner's name.
    let theirs = db::list_bookmarks(&pool, invitee.id, None, false, &none, false)
        .await
        .expect("invitee list");
    assert_eq!(theirs.len(), 2, "unsorted stays the owner's");
    let ids: Vec<Uuid> = theirs.iter().map(|v| v.id).collect();
    let single = one_by_one(&pool, invitee.id, &ids).await;
    assert_eq!(
        json(&theirs.iter().collect::<Vec<_>>()),
        json(&single.iter().collect::<Vec<_>>())
    );
    let both = theirs.iter().find(|v| v.id == in_child_and_secret).unwrap();
    assert!(!both.is_favorite, "flags are per viewer");
    assert_eq!(
        both.categories
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Root / Child"]
    );
    assert!(both.shared_with.is_empty());
    assert!(both.can_write);
    assert_eq!(both.owner_display_name.as_deref(), Some("Owner Person"));
    assert!(theirs.iter().find(|v| v.id == in_root).unwrap().is_unread);

    // Listing one folder gives the same views.
    let folder_list = db::list_bookmarks(&pool, invitee.id, Some(root), false, &none, false)
        .await
        .expect("folder list");
    assert_eq!(
        json(&folder_list.iter().collect::<Vec<_>>()),
        json(
            &theirs
                .iter()
                .filter(|v| v.id == in_root)
                .collect::<Vec<_>>()
        )
    );
}
