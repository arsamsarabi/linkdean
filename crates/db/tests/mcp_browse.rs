//! Empty-`q` MCP browse: folder tree (+ optional tag), no Meili / no vault unlock.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{BookmarkView, CreateBookmark, CreateCategory, Error, User, VaultKeyring};
use linkdean_db as db;
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping mcp browse tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@mcp-browse.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    db::mark_email_verified(pool, user.id)
        .await
        .expect("verify email");
    user
}

async fn category(pool: &PgPool, owner: &User, name: &str, parent_id: Option<Uuid>) -> Uuid {
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
    .expect("category")
    .id
}

async fn bookmark(
    pool: &PgPool,
    owner: &User,
    title: &str,
    folder: Option<Uuid>,
    tags: &[&str],
) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://mcp-browse.example/{}", Uuid::new_v4()),
            title: Some(title.into()),
            description: Some(format!("About {title}")),
            notes: Some(format!("Note on {title}")),
            category_ids: folder.into_iter().collect(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

fn titles(found: &[BookmarkView]) -> Vec<&str> {
    found.iter().map(|b| b.title.as_str()).collect()
}

#[tokio::test]
async fn browse_folder_includes_bookmarks_in_subfolders() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = person(&pool, "owner").await;
    let root = category(&pool, &owner, "Root", None).await;
    let child = category(&pool, &owner, "Child", Some(root)).await;
    let deep = category(&pool, &owner, "Deep", Some(child)).await;
    bookmark(&pool, &owner, "In root", Some(root), &[]).await;
    bookmark(&pool, &owner, "In child", Some(child), &[]).await;
    bookmark(&pool, &owner, "In deep", Some(deep), &[]).await;

    let ids = db::mcp_folder_descendant_ids(&pool, owner.id, root)
        .await
        .expect("descendant ids");
    assert!(ids.contains(&root));
    assert!(ids.contains(&child));
    assert!(ids.contains(&deep));

    let found = db::mcp_browse_bookmarks(&pool, owner.id, Some(root), None, 50)
        .await
        .expect("browse");
    let got = titles(&found);
    assert!(got.contains(&"In root"), "{got:?}");
    assert!(got.contains(&"In child"), "{got:?}");
    assert!(got.contains(&"In deep"), "{got:?}");
}

#[tokio::test]
async fn browse_rejects_vault_folder() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = person(&pool, "vault-owner").await;
    let locked = category(&pool, &owner, "Locked", None).await;
    bookmark(&pool, &owner, "Secret", Some(locked), &[]).await;
    let (vault, _, _) = db::create_vault(&pool, owner.id, "Vault", "vault-passphrase")
        .await
        .expect("vault");
    sqlx::query("UPDATE categories SET is_vault = TRUE, vault_id = $2 WHERE id = $1")
        .bind(locked)
        .bind(vault.id)
        .execute(&pool)
        .await
        .expect("mark vault folder");

    let ids = db::mcp_folder_descendant_ids(&pool, owner.id, locked).await;
    assert!(matches!(ids, Err(Error::BadRequest(_))), "{ids:?}");

    let browse = db::mcp_browse_bookmarks(&pool, owner.id, Some(locked), None, 50).await;
    assert!(matches!(browse, Err(Error::BadRequest(_))), "{browse:?}");
}

#[tokio::test]
async fn browse_tag_is_own_bookmarks_only() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool, "me").await;
    let other = person(&pool, "other").await;
    bookmark(&pool, &me, "Mine A", None, &["design"]).await;
    bookmark(&pool, &me, "Mine B", None, &["Design"]).await;
    bookmark(&pool, &other, "Theirs", None, &["design"]).await;

    let found = db::mcp_browse_bookmarks(&pool, me.id, None, Some("DESIGN"), 50)
        .await
        .expect("tag browse");
    let got = titles(&found);
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got.contains(&"Mine A"), "{got:?}");
    assert!(got.contains(&"Mine B"), "{got:?}");
    assert!(!got.contains(&"Theirs"), "{got:?}");
}
