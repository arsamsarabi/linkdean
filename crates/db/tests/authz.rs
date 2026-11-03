//! Authz regression: user A cannot read/write user B's rows.
//!
//! Requires Postgres. Set `DATABASE_URL` (e.g. `postgres://linkdean:linkdean@127.0.0.1:5432/linkdean`).
//! Skips when unset so CI without a DB stays green.

use linkdean_core::VaultKeyring;
use linkdean_core::{
    CategoryAccess, CreateBookmark, CreateCategory, CreateCategoryShare,
    CreateCategoryShareOutcome, Error, ShareAccess, UpdateBookmark, UpdateCategory,
    UpdateCategoryShare,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn accept_invites(pool: &sqlx::PgPool, invitee_id: Uuid) {
    for invite in db::list_share_invites(pool, invitee_id)
        .await
        .expect("inbox")
    {
        db::accept_share_invite(pool, invitee_id, invite.id)
            .await
            .expect("accept");
    }
}

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping authz tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for authz tests");
    db::migrate(&pool).await.expect("migrate for authz tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@authz.test", Uuid::new_v4())
}

#[tokio::test]
async fn user_b_cannot_access_user_a_bookmark_or_category() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("a"),
        Some("password-a-long"),
        Some("User A"),
        None,
        None,
    )
    .await
    .expect("create user a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");
    let user_b = db::create_user(
        &pool,
        &unique_email("b"),
        Some("password-b-long"),
        Some("User B"),
        None,
        None,
    )
    .await
    .expect("create user b");
    db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("AuthzCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("create category for a");

    let bm = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://authz.example/{}/", Uuid::new_v4()),
            title: Some("Owned by A".into()),
            description: None,
            notes: None,
            category_id: Some(cat.id),
            category_ids: vec![cat.id],
            tags: vec![],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("create bookmark for a");

    // A can read own rows
    assert!(db::get_bookmark_raw(&pool, user_a.id, bm.id).await.is_ok());
    assert!(db::get_category(&pool, user_a.id, cat.id).await.is_ok());

    // B cannot get / update / delete A's bookmark
    assert!(matches!(
        db::get_bookmark_raw(&pool, user_b.id, bm.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::update_bookmark(
            &pool,
            user_b.id,
            bm.id,
            UpdateBookmark {
                url: Some("https://evil.example/".into()),
                title: None,
                description: None,
                notes: None,
                category_id: None,
                category_ids: None,
                tags: None,
                ..Default::default()
            },
            &VaultKeyring::default(),
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::delete_bookmark(&pool, user_b.id, bm.id).await,
        Err(Error::NotFound)
    ));

    // B cannot get / update / delete A's category
    assert!(matches!(
        db::get_category(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::update_category(
            &pool,
            user_b.id,
            cat.id,
            UpdateCategory {
                name: Some("hijacked".into()),
                parent_id: None,
                is_hidden: None,
                position: None,
                icon: None,
                visibility: None,
            },
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::delete_category(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound)
    ));

    // B's list never includes A's bookmark
    let b_list = db::list_bookmarks(
        &pool,
        user_b.id,
        None,
        false,
        &VaultKeyring::default(),
        false,
    )
    .await
    .expect("list for b");
    assert!(!b_list.iter().any(|x| x.id == bm.id));

    // Cleanup A's rows (best-effort; unique emails avoid collisions)
    let _ = db::delete_bookmark(&pool, user_a.id, bm.id).await;
    let _ = db::delete_category(&pool, user_a.id, cat.id).await;
}

#[tokio::test]
async fn share_read_and_write_membership() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("share-a"),
        Some("password-a-long"),
        Some("Share A"),
        None,
        None,
    )
    .await
    .expect("create user a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");
    let user_b = db::create_user(
        &pool,
        &unique_email("share-b"),
        Some("password-b-long"),
        Some("Share B"),
        None,
        None,
    )
    .await
    .expect("create user b");
    db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");
    let user_c = db::create_user(
        &pool,
        &unique_email("share-c"),
        Some("password-c-long"),
        Some("Share C"),
        None,
        None,
    )
    .await
    .expect("create user c");
    db::mark_email_verified(&pool, user_c.id)
        .await
        .expect("verify email");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("SharedCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("create category");

    let bm = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://share.example/{}/", Uuid::new_v4()),
            title: Some("Shared bookmark".into()),
            description: None,
            notes: None,
            category_id: Some(cat.id),
            category_ids: vec![cat.id],
            tags: vec![],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("create bookmark");

    // Outsider C: no access
    assert!(matches!(
        db::get_category(&pool, user_c.id, cat.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::get_bookmark_raw(&pool, user_c.id, bm.id).await,
        Err(Error::NotFound)
    ));

    // A shares with B as read
    let outcome = db::create_share(
        &pool,
        user_a.id,
        cat.id,
        CreateCategoryShare {
            email: user_b.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("create read share");
    assert!(matches!(
        outcome,
        CreateCategoryShareOutcome::Pending { .. }
    ));
    accept_invites(&pool, user_b.id).await;

    let view = db::get_category(&pool, user_b.id, cat.id)
        .await
        .expect("B reads shared cat");
    assert_eq!(view.access, CategoryAccess::Read);

    assert!(db::get_bookmark_raw(&pool, user_b.id, bm.id).await.is_ok());
    assert!(matches!(
        db::update_bookmark(
            &pool,
            user_b.id,
            bm.id,
            UpdateBookmark {
                url: Some("https://share.example/ro-mutate/".into()),
                title: None,
                description: None,
                notes: None,
                category_id: None,
                category_ids: None,
                tags: None,
                ..Default::default()
            },
            &VaultKeyring::default(),
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::delete_bookmark(&pool, user_b.id, bm.id).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::update_category(
            &pool,
            user_b.id,
            cat.id,
            UpdateCategory {
                name: Some("ro-rename".into()),
                parent_id: None,
                is_hidden: None,
                position: None,
                icon: None,
                visibility: None,
            },
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::delete_category(&pool, user_b.id, cat.id).await,
        Err(Error::Forbidden)
    ));

    // Upgrade B to write
    db::update_share(
        &pool,
        user_a.id,
        cat.id,
        user_b.id,
        UpdateCategoryShare {
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("upgrade to write");

    let view = db::get_category(&pool, user_b.id, cat.id)
        .await
        .expect("B write access");
    assert_eq!(view.access, CategoryAccess::Write);

    db::update_bookmark(
        &pool,
        user_b.id,
        bm.id,
        UpdateBookmark {
            title: Some("Edited by B".into()),
            url: None,
            description: None,
            notes: None,
            category_id: None,
            category_ids: None,
            tags: None,
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("RW can edit bookmark");

    let child = db::create_category(
        &pool,
        user_b.id,
        CreateCategory {
            name: format!("ChildByB-{}", Uuid::new_v4()),
            parent_id: Some(cat.id),
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("RW can create child under shared");
    assert_eq!(child.owner_user_id, user_a.id);
    assert_eq!(child.access, CategoryAccess::Write);

    let b_created = db::create_bookmark(
        &pool,
        user_b.id,
        CreateBookmark {
            url: format!("https://share.example/b/{}/", Uuid::new_v4()),
            title: Some("B added".into()),
            description: None,
            notes: None,
            category_id: Some(cat.id),
            category_ids: vec![cat.id],
            tags: vec![],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("RW can add bookmark");

    // C still blocked
    assert!(matches!(
        db::get_bookmark_raw(&pool, user_c.id, bm.id).await,
        Err(Error::NotFound)
    ));
    assert!(!db::list_bookmarks(
        &pool,
        user_c.id,
        None,
        false,
        &VaultKeyring::default(),
        false
    )
    .await
    .expect("list c")
    .iter()
    .any(|x| x.id == bm.id || x.id == b_created.id));

    // B sees shared bookmark in list
    assert!(db::list_bookmarks(
        &pool,
        user_b.id,
        None,
        false,
        &VaultKeyring::default(),
        false
    )
    .await
    .expect("list b")
    .iter()
    .any(|x| x.id == bm.id));

    // Revoke → B loses access
    db::delete_share(&pool, user_a.id, cat.id, user_b.id)
        .await
        .expect("revoke");
    assert!(matches!(
        db::get_category(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::get_bookmark_raw(&pool, user_b.id, bm.id).await,
        Err(Error::NotFound)
    ));

    let _ = db::delete_bookmark(&pool, user_a.id, bm.id).await;
    let _ = db::delete_bookmark(&pool, user_b.id, b_created.id).await;
    let _ = db::delete_category(&pool, user_a.id, child.id).await;
    let _ = db::delete_category(&pool, user_a.id, cat.id).await;
}

#[tokio::test]
async fn cannot_share_vault_category() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("vault-a"),
        Some("password-a-long"),
        Some("Vault A"),
        None,
        None,
    )
    .await
    .expect("user a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");
    let user_b = db::create_user(
        &pool,
        &unique_email("vault-b"),
        Some("password-b-long"),
        Some("Vault B"),
        None,
        None,
    )
    .await
    .expect("user b");
    db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");

    let (vault, _, _) = db::create_vault(&pool, user_a.id, "Vault", "vault-passphrase")
        .await
        .expect("vault");
    let vault_cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("VaultCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: Some(vault.id),
            icon: None,
        },
    )
    .await
    .expect("vault cat");

    assert!(matches!(
        db::create_share(
            &pool,
            user_a.id,
            vault_cat.id,
            CreateCategoryShare {
                email: user_b.email.clone(),
                access: ShareAccess::Read,
            },
        )
        .await,
        Err(Error::BadRequest(_))
    ));

    // Unknown email on vault still BadRequest (pending not allowed for vault).
    assert!(matches!(
        db::create_share(
            &pool,
            user_a.id,
            vault_cat.id,
            CreateCategoryShare {
                email: format!("nobody-{}@authz.test", Uuid::new_v4()),
                access: ShareAccess::Read,
            },
        )
        .await,
        Err(Error::BadRequest(_))
    ));

    assert!(matches!(
        db::enable_category_public_link(&pool, user_a.id, vault_cat.id).await,
        Err(Error::BadRequest(_))
    ));

    let _ = db::delete_category(&pool, user_a.id, vault_cat.id).await;
}

#[tokio::test]
async fn public_list_slug_is_unlisted_and_revocable() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("pub-a"),
        Some("password-a-long"),
        Some("Public A"),
        None,
        None,
    )
    .await
    .expect("user a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");
    let user_b = db::create_user(
        &pool,
        &unique_email("pub-b"),
        Some("password-b-long"),
        Some("Public B"),
        None,
        None,
    )
    .await
    .expect("user b");
    db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("PublicList-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("cat");

    let bm = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://public.example/{}/", Uuid::new_v4()),
            title: Some("Public BM".into()),
            description: Some("desc".into()),
            notes: Some("secret notes".into()),
            category_id: Some(cat.id),
            category_ids: vec![cat.id],
            tags: vec!["tag-a".into()],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark");

    // Stranger cannot list via normal ACL
    assert!(matches!(
        db::get_category(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound)
    ));

    let link = db::enable_category_public_link(&pool, user_a.id, cat.id)
        .await
        .expect("enable public");
    assert!(link.enabled);
    let slug = link.slug.expect("slug");

    let pub_list = db::get_public_list_by_slug(&pool, &slug)
        .await
        .expect("resolve public");
    assert_eq!(pub_list.name, cat.name);
    assert_eq!(pub_list.owner_display_name, "Public A");
    assert_eq!(pub_list.bookmarks.len(), 1);
    assert_eq!(pub_list.bookmarks[0].id, bm.id);
    assert_eq!(pub_list.bookmarks[0].title, "Public BM");
    // notes never appear on PublicBookmarkView

    let only_me = |is_private| UpdateBookmark {
        url: None,
        title: None,
        description: None,
        notes: None,
        category_id: None,
        category_ids: None,
        tags: None,
        is_favorite: None,
        is_unread: None,
        is_pinned: None,
        is_private: Some(is_private),
        add_tags: None,
        add_category_ids: None,
    };
    db::update_bookmark(
        &pool,
        user_a.id,
        bm.id,
        only_me(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("set private");
    let hidden = db::get_public_list_by_slug(&pool, &slug)
        .await
        .expect("resolve public");
    assert!(
        hidden.bookmarks.is_empty(),
        "\"only me\" must stay off the public page"
    );
    db::update_bookmark(
        &pool,
        user_a.id,
        bm.id,
        only_me(false),
        &VaultKeyring::default(),
    )
    .await
    .expect("clear private");

    let rotated = db::rotate_category_public_link(&pool, user_a.id, cat.id)
        .await
        .expect("rotate");
    let new_slug = rotated.slug.expect("new slug");
    assert_ne!(new_slug, slug);
    assert!(matches!(
        db::get_public_list_by_slug(&pool, &slug).await,
        Err(Error::NotFound)
    ));
    assert!(db::get_public_list_by_slug(&pool, &new_slug).await.is_ok());

    db::disable_category_public_link(&pool, user_a.id, cat.id)
        .await
        .expect("disable");
    assert!(matches!(
        db::get_public_list_by_slug(&pool, &new_slug).await,
        Err(Error::NotFound)
    ));

    // Non-owner cannot manage link
    assert!(matches!(
        db::enable_category_public_link(&pool, user_b.id, cat.id).await,
        Err(Error::NotFound | Error::Forbidden)
    ));

    let _ = db::delete_bookmark(&pool, user_a.id, bm.id).await;
    let _ = db::delete_category(&pool, user_a.id, cat.id).await;
}

#[tokio::test]
async fn pending_share_invite_claims_on_register() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("pend-a"),
        Some("password-a-long"),
        Some("Pend A"),
        None,
        None,
    )
    .await
    .expect("user a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("PendList-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("cat");

    let pending_email = unique_email("pend-invitee");
    let outcome = db::create_share(
        &pool,
        user_a.id,
        cat.id,
        CreateCategoryShare {
            email: pending_email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("pending invite");
    let CreateCategoryShareOutcome::Pending { invite } = outcome else {
        panic!("expected pending invite for unknown email");
    };
    assert_eq!(invite.invitee_email, pending_email);
    assert_eq!(invite.access, ShareAccess::Write);

    let user_b = db::create_user(
        &pool,
        &pending_email,
        Some("password-b-long"),
        Some("Pend B"),
        None,
        None,
    )
    .await
    .expect("user b");

    // Verifying the address attaches the invite waiting for it.
    let claimed = db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");
    assert_eq!(claimed, 1);
    accept_invites(&pool, user_b.id).await;

    let view = db::get_category(&pool, user_b.id, cat.id)
        .await
        .expect("claimed access");
    assert_eq!(view.access, CategoryAccess::Write);

    let pending_left = db::list_pending_for_subject(
        &pool,
        user_a.id,
        linkdean_core::PendingShareSubjectType::Category,
        cat.id,
    )
    .await
    .expect("list pending");
    assert!(pending_left.is_empty());

    let _ = db::delete_category(&pool, user_a.id, cat.id).await;
}

#[tokio::test]
async fn public_list_owner_without_a_name_shows_no_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = db::create_user(
        &pool,
        &format!("leaky-{}@authz.test", Uuid::new_v4()),
        Some("password-long"),
        None,
        None,
        None,
    )
    .await
    .expect("user");
    db::mark_email_verified(&pool, owner.id)
        .await
        .expect("verify email");
    let cat = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: format!("Pub-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category");
    let slug = db::enable_category_public_link(&pool, owner.id, cat.id)
        .await
        .expect("enable")
        .slug
        .expect("slug");

    let list = db::get_public_list_by_slug(&pool, &slug)
        .await
        .expect("resolve");
    assert_eq!(list.owner_display_name, "Owner");
    assert!(!list.owner_display_name.contains("leaky"));
}
