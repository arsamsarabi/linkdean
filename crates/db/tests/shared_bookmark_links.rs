//! What a list invitee can do to, and see of, the owner's bookmark links.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, Error, ShareAccess, UpdateBookmark, User,
    VaultKeyring,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping shared bookmark link tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@links.test", Uuid::new_v4());
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

async fn share(
    pool: &PgPool,
    owner: &User,
    category_id: Uuid,
    invitee: &User,
    access: ShareAccess,
) {
    db::create_share(
        pool,
        owner.id,
        category_id,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access,
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

async fn links(pool: &PgPool, bookmark_id: Uuid) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT category_id FROM bookmark_categories WHERE bookmark_id = $1")
            .bind(bookmark_id)
            .fetch_all(pool)
            .await
            .expect("links");
    ids.sort();
    ids
}

struct Setup {
    pool: PgPool,
    owner: User,
    invitee: User,
    work: Uuid,
    work_sub: Uuid,
    private: Uuid,
    bookmark: Uuid,
}

/// Owner files one bookmark in a shared list and a private one. The invitee can write
/// the shared list.
async fn setup() -> Option<Setup> {
    let pool = pool_or_skip().await?;
    let owner = person(&pool, "owner").await;
    let invitee = person(&pool, "invitee").await;
    let parent = category(&pool, &owner, "Clients", None).await;
    let work = category(&pool, &owner, "Work", Some(parent)).await;
    let work_sub = category(&pool, &owner, "Sub", Some(work)).await;
    let private = category(&pool, &owner, "Private plans", None).await;
    let bookmark = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: format!("https://links.example/{}", Uuid::new_v4()),
            category_ids: vec![work, private],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id;
    share(&pool, &owner, work, &invitee, ShareAccess::Write).await;
    Some(Setup {
        pool,
        owner,
        invitee,
        work,
        work_sub,
        private,
        bookmark,
    })
}

async fn view(pool: &PgPool, viewer: &User, id: Uuid) -> linkdean_core::BookmarkView {
    let raw = db::get_bookmark_raw(pool, viewer.id, id)
        .await
        .expect("raw");
    db::bookmark_to_view(pool, raw, &VaultKeyring::default(), viewer.id)
        .await
        .expect("view")
        .expect("visible")
}

fn move_to(ids: Vec<Uuid>) -> UpdateBookmark {
    UpdateBookmark {
        category_ids: Some(ids),
        ..Default::default()
    }
}

#[tokio::test]
async fn an_invitee_cannot_file_the_owners_bookmark_in_their_own_list() {
    let Some(s) = setup().await else {
        return;
    };
    let mine = category(&s.pool, &s.invitee, "Mine", None).await;

    let got = db::update_bookmark(
        &s.pool,
        s.invitee.id,
        s.bookmark,
        move_to(vec![mine]),
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(got, Err(Error::Forbidden)), "{got:?}");

    let mut want = vec![s.work, s.private];
    want.sort();
    assert_eq!(links(&s.pool, s.bookmark).await, want);
}

#[tokio::test]
async fn an_invitee_moving_within_the_shared_list_keeps_the_owners_other_links() {
    let Some(s) = setup().await else {
        return;
    };

    db::update_bookmark(
        &s.pool,
        s.invitee.id,
        s.bookmark,
        move_to(vec![s.work_sub]),
        &VaultKeyring::default(),
    )
    .await
    .expect("move");

    let mut want = vec![s.work_sub, s.private];
    want.sort();
    assert_eq!(links(&s.pool, s.bookmark).await, want);
}

#[tokio::test]
async fn an_invitee_sees_only_shared_lists_with_paths_from_the_shared_root() {
    let Some(s) = setup().await else {
        return;
    };

    let shown = view(&s.pool, &s.invitee, s.bookmark).await;
    let seen: Vec<(Uuid, String)> = shown
        .categories
        .iter()
        .map(|c| (c.id, c.path.clone()))
        .collect();
    assert_eq!(seen, vec![(s.work, "Work".to_string())]);
    assert_eq!(shown.category_id, Some(s.work));

    let owner_view = view(&s.pool, &s.owner, s.bookmark).await;
    assert!(owner_view
        .categories
        .iter()
        .any(|c| c.path == "Clients / Work"));
    assert!(owner_view.categories.iter().any(|c| c.id == s.private));
}
