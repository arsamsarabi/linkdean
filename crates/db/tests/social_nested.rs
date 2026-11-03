//! Likes and comments below a share root, and who may delete a comment.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, CreateSocialComment, Error, ShareAccess,
    SocialSubjectType, User, VaultKeyring,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping nested social tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@social.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    db::mark_email_verified(pool, user.id)
        .await
        .expect("verify email");
    user
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

async fn share(pool: &PgPool, owner: &User, folder: Uuid, invitee: &User) {
    db::create_share(
        pool,
        owner.id,
        folder,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Write,
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

async fn bookmark(pool: &PgPool, owner: &User, folder: Uuid) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://social.example/{}", Uuid::new_v4()),
            title: Some("Shared".into()),
            category_id: Some(folder),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

#[tokio::test]
async fn invitees_like_and_comment_below_the_share_root() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let root = folder(&pool, &owner, None).await;
    let sub = folder(&pool, &owner, Some(root)).await;
    share(&pool, &owner, root, &invitee).await;
    let b = bookmark(&pool, &owner, sub).await;

    db::set_like(&pool, invitee.id, SocialSubjectType::Bookmark, b, true)
        .await
        .expect("like bookmark in subfolder");
    db::set_like(&pool, invitee.id, SocialSubjectType::Category, sub, true)
        .await
        .expect("like subfolder");
    db::add_comment(
        &pool,
        invitee.id,
        SocialSubjectType::Bookmark,
        b,
        CreateSocialComment {
            body: "Nice".into(),
        },
    )
    .await
    .expect("comment");

    // Nothing shared: still refused.
    let lone = folder(&pool, &owner, None).await;
    let refused = db::set_like(&pool, owner.id, SocialSubjectType::Category, lone, true).await;
    assert!(matches!(refused, Err(Error::BadRequest(_))), "{refused:?}");
}

async fn comment_id(pool: &PgPool, viewer: &User, b: Uuid) -> Uuid {
    db::get_social_summary(pool, viewer.id, SocialSubjectType::Bookmark, b)
        .await
        .expect("summary")
        .comments[0]
        .id
}

#[tokio::test]
async fn authors_and_owners_can_delete_comments() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let other = user(&pool, "other").await;
    let root = folder(&pool, &owner, None).await;
    share(&pool, &owner, root, &invitee).await;
    share(&pool, &owner, root, &other).await;
    let b = bookmark(&pool, &owner, root).await;
    let say = |who: Uuid| {
        let pool = pool.clone();
        async move {
            db::add_comment(
                &pool,
                who,
                SocialSubjectType::Bookmark,
                b,
                CreateSocialComment { body: "Hi".into() },
            )
            .await
            .expect("comment");
        }
    };

    // The owner sees a delete action on someone else's comment and may use it.
    say(invitee.id).await;
    let summary = db::get_social_summary(&pool, owner.id, SocialSubjectType::Bookmark, b)
        .await
        .expect("summary");
    assert!(summary.comments[0].can_delete);
    let id = comment_id(&pool, &owner, b).await;
    let stranger = db::delete_comment(&pool, other.id, id).await;
    assert!(matches!(stranger, Err(Error::Forbidden)), "{stranger:?}");
    db::delete_comment(&pool, owner.id, id)
        .await
        .expect("owner deletes");

    // The author may delete after losing access.
    say(invitee.id).await;
    let id = comment_id(&pool, &invitee, b).await;
    db::delete_share(&pool, owner.id, root, invitee.id)
        .await
        .expect("revoke");
    db::delete_comment(&pool, invitee.id, id)
        .await
        .expect("author deletes");
}
