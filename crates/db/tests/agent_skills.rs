//! Which bookmarks an Agent Skill made from a folder or a tag holds, and for whom.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, Error, ShareAccess, User, VaultKeyring,
};
use linkdean_db::{self as db, SkillFrom};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping agent skill tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@skills.test", Uuid::new_v4());
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
            url: format!("https://skills.example/{}", Uuid::new_v4()),
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

async fn share_read(pool: &PgPool, owner: &User, category_id: Uuid, invitee: &User) {
    db::create_share(
        pool,
        owner.id,
        category_id,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("share");
    for invite in db::list_share_invites(pool, invitee.id)
        .await
        .expect("inbox")
    {
        db::accept_share_invite(pool, invitee.id, invite.id)
            .await
            .expect("accept");
    }
}

fn titles(found: &db::SkillBookmarks) -> Vec<String> {
    found.links.iter().map(|l| l.title.clone()).collect()
}

#[tokio::test]
async fn a_folder_skill_holds_its_subfolders_but_not_vault_deleted_or_private_bookmarks() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = person(&pool, "owner").await;
    let reader = person(&pool, "reader").await;
    let stranger = person(&pool, "stranger").await;
    let root = category(&pool, &owner, "Rust", None).await;
    let web = category(&pool, &owner, "Web", Some(root)).await;
    let deep = category(&pool, &owner, "Axum", Some(web)).await;
    let locked = category(&pool, &owner, "Locked", Some(root)).await;
    bookmark(&pool, &owner, "Secret", Some(locked), &[]).await;
    // A vault folder nested under the shared one, as an old layout could leave it.
    let (vault, _, _) = db::create_vault(&pool, owner.id, "Vault", "vault-passphrase")
        .await
        .expect("vault");
    sqlx::query("UPDATE categories SET is_vault = TRUE, vault_id = $2 WHERE id = $1")
        .bind(locked)
        .bind(vault.id)
        .execute(&pool)
        .await
        .expect("vault folder");

    bookmark(&pool, &owner, "Tokio", Some(root), &["async"]).await;
    bookmark(&pool, &owner, "Hyper", Some(web), &["http"]).await;
    bookmark(&pool, &owner, "Axum guide", Some(deep), &["http", "web"]).await;
    let gone = bookmark(&pool, &owner, "Deleted", Some(root), &[]).await;
    db::delete_bookmark(&pool, owner.id, gone)
        .await
        .expect("delete");
    let only_me = bookmark(&pool, &owner, "Only me", Some(root), &[]).await;
    sqlx::query("UPDATE bookmarks SET is_private = TRUE WHERE id = $1")
        .bind(only_me)
        .execute(&pool)
        .await
        .expect("private");

    let mine = db::agent_skill_bookmarks(&pool, owner.id, &SkillFrom::Folder(root))
        .await
        .expect("owner skill");
    assert_eq!(mine.source.label, "Rust");
    assert_eq!(
        titles(&mine),
        ["Only me", "Tokio", "Hyper", "Axum guide"],
        "the folder's own links first, then subfolders by path"
    );
    let axum = &mine.links[3];
    assert_eq!(axum.path, ["Web", "Axum"]);
    assert_eq!(axum.tags, ["http", "web"]);
    assert_eq!(axum.notes.as_deref(), Some("Note on Axum guide"));
    assert_eq!(mine.truncated, 0);

    share_read(&pool, &owner, root, &reader).await;
    let theirs = db::agent_skill_bookmarks(&pool, reader.id, &SkillFrom::Folder(root))
        .await
        .expect("reader skill");
    assert_eq!(titles(&theirs), ["Tokio", "Hyper", "Axum guide"]);
    assert!(
        theirs.links.iter().all(|l| l.notes.is_none()),
        "someone else's notes stay private"
    );

    let refused = db::agent_skill_bookmarks(&pool, stranger.id, &SkillFrom::Folder(root)).await;
    assert!(matches!(refused, Err(Error::NotFound)));
    let vault = db::agent_skill_bookmarks(&pool, owner.id, &SkillFrom::Folder(locked)).await;
    assert!(matches!(vault, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn a_tag_skill_holds_only_the_users_own_bookmarks() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool, "me").await;
    let other = person(&pool, "other").await;
    bookmark(&pool, &me, "B page", None, &["Design"]).await;
    bookmark(&pool, &me, "A page", None, &["design", "css"]).await;
    bookmark(&pool, &me, "Untagged", None, &[]).await;
    bookmark(&pool, &other, "Theirs", None, &["design"]).await;

    let found = db::agent_skill_bookmarks(&pool, me.id, &SkillFrom::Tag("DESIGN".into()))
        .await
        .expect("tag skill");
    assert_eq!(titles(&found), ["A page", "B page"]);
    assert_eq!(
        found.source.label, "Design",
        "capitals sort first, so the spelling is stable"
    );
    assert!(found.links.iter().all(|l| l.path.is_empty()));

    let none = db::agent_skill_bookmarks(&pool, me.id, &SkillFrom::Tag("nothing".into())).await;
    assert!(matches!(none, Err(Error::NotFound)));
}
