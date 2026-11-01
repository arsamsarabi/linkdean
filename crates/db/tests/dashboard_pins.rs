//! Pinning a shared dashboard as the viewer's default, and showing its pins' images.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
use linkdean_core::{
    CreateBookmark, CreateDashboard, CreateDashboardShare, Dashboard, DashboardView, Error,
    PatchDashboard, PutDashboardLayout, PutDashboardSection, ShareAccess, User,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
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

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping dashboard pin tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@pins.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn shared_board(pool: &PgPool, owner: &User, invitee: &User, access: ShareAccess) -> Uuid {
    let board = db::create_dashboard(
        pool,
        owner.id,
        CreateDashboard {
            title: Some("Shared".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard");
    db::create_dashboard_share(
        pool,
        owner.id,
        board.id,
        CreateDashboardShare {
            email: invitee.email.clone(),
            access,
        },
    )
    .await
    .expect("share");
    accept_invites(pool, invitee.id).await;
    board.id
}

fn pin(value: bool) -> PatchDashboard {
    PatchDashboard {
        title: None,
        is_default: Some(value),
    }
}

async fn default_of(pool: &PgPool, user: &User) -> DashboardView {
    db::get_default_dashboard(pool, user.id, &VaultKeyring::default())
        .await
        .expect("default")
}

async fn own_default_row(pool: &PgPool, user: &User) -> Dashboard {
    db::ensure_default_dashboard(pool, user.id)
        .await
        .expect("own default")
}

async fn defaults_in_list(pool: &PgPool, user: &User) -> Vec<Uuid> {
    db::list_dashboards(pool, user.id)
        .await
        .expect("list")
        .into_iter()
        .filter(|d| d.is_default)
        .map(|d| d.id)
        .collect()
}

#[tokio::test]
async fn read_invitee_can_pin_a_shared_dashboard() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;

    let view = db::patch_dashboard(
        &pool,
        invitee.id,
        board,
        pin(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("pin");

    assert!(view.is_default);
    assert_eq!(default_of(&pool, &invitee).await.id, board);
    assert_eq!(defaults_in_list(&pool, &invitee).await, vec![board]);
}

#[tokio::test]
async fn pin_is_private_to_the_invitee() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    let owner_default = own_default_row(&pool, &owner).await.id;

    db::patch_dashboard(
        &pool,
        invitee.id,
        board,
        pin(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("pin");

    assert_eq!(default_of(&pool, &owner).await.id, owner_default);
    let owner_view = db::get_dashboard(&pool, owner.id, board, &VaultKeyring::default())
        .await
        .expect("owner view");
    assert!(!owner_view.is_default);
}

#[tokio::test]
async fn revoking_access_returns_to_own_default() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let own = own_default_row(&pool, &invitee).await.id;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    db::patch_dashboard(
        &pool,
        invitee.id,
        board,
        pin(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("pin");

    db::delete_dashboard_share(&pool, owner.id, board, invitee.id)
        .await
        .expect("revoke");

    assert_eq!(default_of(&pool, &invitee).await.id, own);
    assert_eq!(defaults_in_list(&pool, &invitee).await, vec![own]);
}

#[tokio::test]
async fn deleting_or_blocking_returns_to_own_default() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let own = own_default_row(&pool, &invitee).await.id;

    let deleted = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    db::patch_dashboard(
        &pool,
        invitee.id,
        deleted,
        pin(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("pin");
    db::delete_dashboard(&pool, owner.id, deleted)
        .await
        .expect("delete");
    assert_eq!(default_of(&pool, &invitee).await.id, own);

    let blocked = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    db::patch_dashboard(
        &pool,
        invitee.id,
        blocked,
        pin(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("pin");
    db::block_user(&pool, owner.id, invitee.id)
        .await
        .expect("block");
    assert_eq!(default_of(&pool, &invitee).await.id, own);
}

#[tokio::test]
async fn pinning_own_dashboard_replaces_a_shared_pin() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let own = own_default_row(&pool, &invitee).await.id;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    db::patch_dashboard(
        &pool,
        invitee.id,
        board,
        pin(true),
        &VaultKeyring::default(),
    )
    .await
    .expect("pin shared");

    let view = db::patch_dashboard(&pool, invitee.id, own, pin(true), &VaultKeyring::default())
        .await
        .expect("pin own");

    assert!(view.is_default);
    assert_eq!(default_of(&pool, &invitee).await.id, own);
    assert_eq!(defaults_in_list(&pool, &invitee).await, vec![own]);
}

#[tokio::test]
async fn second_shared_pin_replaces_the_first() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let first = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    let second = shared_board(&pool, &owner, &invitee, ShareAccess::Write).await;

    for board in [first, second] {
        db::patch_dashboard(
            &pool,
            invitee.id,
            board,
            pin(true),
            &VaultKeyring::default(),
        )
        .await
        .expect("pin");
    }

    assert_eq!(defaults_in_list(&pool, &invitee).await, vec![second]);
}

#[tokio::test]
async fn pinning_does_not_grant_rename_or_access() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let stranger = user(&pool, "stranger").await;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;

    let rename = db::patch_dashboard(
        &pool,
        invitee.id,
        board,
        PatchDashboard {
            title: Some("Renamed".into()),
            is_default: Some(true),
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(rename, Err(Error::Forbidden)));

    let outsider = db::patch_dashboard(
        &pool,
        stranger.id,
        board,
        pin(true),
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(outsider, Err(Error::NotFound)));
}

async fn pinned_bookmark(pool: &PgPool, owner: &User, board: Uuid) -> Uuid {
    let bookmark = db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://pins.example/{}", Uuid::new_v4()),
            title: Some("Pinned".into()),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark");
    db::put_dashboard(
        pool,
        owner.id,
        board,
        PutDashboardLayout {
            title: None,
            sections: vec![PutDashboardSection {
                title: "Pins".into(),
                bookmark_ids: vec![bookmark.id],
            }],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("layout");
    bookmark.id
}

#[tokio::test]
async fn invitee_can_load_images_for_dashboard_pins_only_while_shared() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let stranger = user(&pool, "stranger").await;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    let bookmark = pinned_bookmark(&pool, &owner, board).await;

    db::get_bookmark_for_assets(&pool, invitee.id, bookmark)
        .await
        .expect("invitee image");
    let outsider = db::get_bookmark_for_assets(&pool, stranger.id, bookmark).await;
    assert!(matches!(outsider, Err(Error::NotFound)));

    // Images only: full bookmark reads stay limited to folder shares.
    let full = db::get_bookmark_raw(&pool, invitee.id, bookmark).await;
    assert!(matches!(full, Err(Error::NotFound)));

    db::delete_dashboard_share(&pool, owner.id, board, invitee.id)
        .await
        .expect("revoke");
    let revoked = db::get_bookmark_for_assets(&pool, invitee.id, bookmark).await;
    assert!(matches!(revoked, Err(Error::NotFound)));
}

#[tokio::test]
async fn vault_pins_stay_hidden_from_invitees() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let board = shared_board(&pool, &owner, &invitee, ShareAccess::Read).await;
    let bookmark = pinned_bookmark(&pool, &owner, board).await;
    db::setup_vault(&pool, owner.id, "vault-passphrase")
        .await
        .expect("vault");
    let vault = db::get_vault(&pool, owner.id).await.expect("get vault");
    sqlx::query("UPDATE bookmarks SET is_vault = TRUE, vault_id = $2 WHERE id = $1")
        .bind(bookmark)
        .bind(vault.id)
        .execute(&pool)
        .await
        .expect("mark vault");

    let denied = db::get_bookmark_for_assets(&pool, invitee.id, bookmark).await;
    assert!(matches!(denied, Err(Error::NotFound)));
}
