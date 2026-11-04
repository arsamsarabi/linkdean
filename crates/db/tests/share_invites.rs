//! Sharing sends an invite. Access starts only when the invitee accepts.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
use linkdean_core::{
    CreateCategory, CreateCategoryShare, CreateCategoryShareOutcome, CreateDashboard,
    CreateDashboardShare, CreateDashboardShareOutcome, Error, PendingShareSubjectType, ShareAccess,
    User,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping share invite tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@invites.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    db::mark_email_verified(pool, user.id)
        .await
        .expect("verify email");
    user
}

async fn category(pool: &PgPool, owner: &User) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Invites-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn invite(
    pool: &PgPool,
    owner: &User,
    category_id: Uuid,
    email: &str,
) -> linkdean_core::Result<CreateCategoryShareOutcome> {
    db::create_share(
        pool,
        owner.id,
        category_id,
        CreateCategoryShare {
            email: email.to_string(),
            access: ShareAccess::Read,
        },
    )
    .await
}

async fn only_invite(pool: &PgPool, invitee: &User) -> Uuid {
    db::list_share_invites(pool, invitee.id)
        .await
        .expect("inbox")[0]
        .id
}

#[tokio::test]
async fn invite_grants_nothing_until_accepted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;

    let outcome = invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    assert!(matches!(
        outcome,
        CreateCategoryShareOutcome::Pending { .. }
    ));
    assert!(db::get_category(&pool, invitee.id, cat).await.is_err());

    let id = only_invite(&pool, &invitee).await;
    db::accept_share_invite(&pool, invitee.id, id)
        .await
        .expect("accept");

    assert!(db::get_category(&pool, invitee.id, cat).await.is_ok());
    assert!(db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox")
        .is_empty());
    let again = db::accept_share_invite(&pool, invitee.id, id).await;
    assert!(matches!(again, Err(Error::NotFound)));
}

#[tokio::test]
async fn dashboard_invite_grants_nothing_until_accepted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let board = db::create_dashboard(
        &pool,
        owner.id,
        CreateDashboard {
            title: Some("Shared".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard");

    let outcome = db::create_dashboard_share(
        &pool,
        owner.id,
        board.id,
        CreateDashboardShare {
            email: invitee.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("invite");
    assert!(matches!(
        outcome,
        CreateDashboardShareOutcome::Pending { .. }
    ));
    assert!(
        db::get_dashboard(&pool, invitee.id, board.id, &VaultKeyring::default())
            .await
            .is_err()
    );

    let id = only_invite(&pool, &invitee).await;
    db::accept_share_invite(&pool, invitee.id, id)
        .await
        .expect("accept");
    assert!(
        db::get_dashboard(&pool, invitee.id, board.id, &VaultKeyring::default())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn rejecting_is_silent_blocks_reinvite_and_can_be_restored() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    let id = only_invite(&pool, &invitee).await;

    db::reject_share_invite(&pool, invitee.id, id)
        .await
        .expect("reject");

    let pending =
        db::list_pending_for_subject(&pool, owner.id, PendingShareSubjectType::Category, cat)
            .await
            .expect("owner list");
    assert!(pending.is_empty());
    assert!(db::list_notifications(&pool, owner.id, 20)
        .await
        .expect("owner notifications")
        .is_empty());

    let denied = invite(&pool, &owner, cat, &invitee.email).await;
    assert!(matches!(denied, Err(Error::Conflict(_))));

    let inbox = db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox");
    assert!(inbox[0].rejected_at.is_some());
    assert!(matches!(
        db::accept_share_invite(&pool, invitee.id, id).await,
        Err(Error::NotFound)
    ));

    db::restore_share_invite(&pool, invitee.id, id)
        .await
        .expect("restore");
    assert!(db::get_category(&pool, invitee.id, cat).await.is_err());
    db::accept_share_invite(&pool, invitee.id, id)
        .await
        .expect("accept");
    assert!(db::get_category(&pool, invitee.id, cat).await.is_ok());
}

#[tokio::test]
async fn other_users_cannot_act_on_an_invite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let stranger = user(&pool, "stranger").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    let id = only_invite(&pool, &invitee).await;

    for result in [
        db::accept_share_invite(&pool, stranger.id, id).await,
        db::reject_share_invite(&pool, stranger.id, id).await,
        db::accept_share_invite(&pool, owner.id, id).await,
    ] {
        assert!(matches!(result, Err(Error::NotFound)));
    }
}

#[tokio::test]
async fn verified_signup_links_the_invite_without_granting() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("later-{}@invites.test", Uuid::new_v4());
    invite(&pool, &owner, cat, &email).await.expect("invite");

    let later = db::create_user(&pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("register");
    let linked = db::link_pending_share_invites(&pool, &email, later.id)
        .await
        .expect("link");
    assert_eq!(linked, 1);

    assert!(db::get_category(&pool, later.id, cat).await.is_err());
    assert_eq!(
        db::list_share_invites(&pool, later.id)
            .await
            .expect("inbox")
            .len(),
        1
    );
}

#[tokio::test]
async fn changing_email_keeps_the_invite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    sqlx::query("UPDATE users SET email = $2 WHERE id = $1")
        .bind(invitee.id)
        .bind(format!("moved-{}@invites.test", Uuid::new_v4()))
        .execute(&pool)
        .await
        .expect("change email");

    let id = only_invite(&pool, &invitee).await;
    db::accept_share_invite(&pool, invitee.id, id)
        .await
        .expect("accept");
    assert!(db::get_category(&pool, invitee.id, cat).await.is_ok());
}

#[tokio::test]
async fn blocking_revokes_open_invites_including_rejected() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    let id = only_invite(&pool, &invitee).await;
    db::reject_share_invite(&pool, invitee.id, id)
        .await
        .expect("reject");

    db::block_user(&pool, invitee.id, owner.id)
        .await
        .expect("block");
    assert!(db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox")
        .is_empty());
}

#[tokio::test]
async fn accepting_after_the_subject_is_gone_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    let id = only_invite(&pool, &invitee).await;

    db::delete_category(&pool, owner.id, cat)
        .await
        .expect("delete");
    assert!(db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox")
        .is_empty());
    assert!(matches!(
        db::accept_share_invite(&pool, invitee.id, id).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn reinviting_updates_access_without_notifying_again() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    db::create_share(
        &pool,
        owner.id,
        cat,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("reinvite");

    let inbox = db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox");
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].access, ShareAccess::Write);
    let notifications = db::list_notifications(&pool, invitee.id, 20)
        .await
        .expect("notifications");
    assert_eq!(notifications.len(), 1);
}

#[tokio::test]
async fn sharing_again_after_accepting_updates_the_share() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    let id = only_invite(&pool, &invitee).await;
    db::accept_share_invite(&pool, invitee.id, id)
        .await
        .expect("accept");

    let outcome = db::create_share(
        &pool,
        owner.id,
        cat,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("change access");
    let CreateCategoryShareOutcome::Active { share } = outcome else {
        panic!("expected active share");
    };
    assert_eq!(share.access, ShareAccess::Write);
}

#[tokio::test]
async fn owner_can_cancel_an_invite_and_send_a_new_one() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let invitee = user(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;
    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("invite");
    let id = only_invite(&pool, &invitee).await;

    db::revoke_pending_share_invite(&pool, owner.id, PendingShareSubjectType::Category, cat, id)
        .await
        .expect("cancel");
    assert!(db::list_share_invites(&pool, invitee.id)
        .await
        .expect("inbox")
        .is_empty());
    assert!(matches!(
        db::accept_share_invite(&pool, invitee.id, id).await,
        Err(Error::NotFound)
    ));

    invite(&pool, &owner, cat, &invitee.email)
        .await
        .expect("new invite");
    assert_eq!(
        db::list_share_invites(&pool, invitee.id)
            .await
            .expect("inbox")
            .len(),
        1
    );
}

async fn pending_claim(pool: &PgPool, owner: &User, category_id: Uuid, email: &str) -> String {
    match invite(pool, owner, category_id, email)
        .await
        .expect("invite")
    {
        CreateCategoryShareOutcome::Pending { invite } => invite
            .claim_token
            .expect("invite to a new address has a token"),
        other => panic!("expected a pending invite, got {other:?}"),
    }
}

async fn register(pool: &PgPool, email: &str) -> User {
    db::create_user(pool, email, Some("password-long"), None, None, None)
        .await
        .expect("register")
}

#[tokio::test]
async fn registering_with_an_invited_email_does_not_claim_the_invite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("target-{}@invites.test", Uuid::new_v4());
    pending_claim(&pool, &owner, cat, &email).await;

    let squatter = register(&pool, &email).await;
    let claimed = db::claim_pending_share_invite(&pool, "not-the-token", &email, squatter.id)
        .await
        .expect("claim");

    assert!(!claimed);
    assert!(db::list_share_invites(&pool, squatter.id)
        .await
        .expect("inbox")
        .is_empty());
}

#[tokio::test]
async fn claim_link_works_once_and_only_for_its_address() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    let token = pending_claim(&pool, &owner, cat, &email).await;

    let other = user(&pool, "other").await;
    assert!(
        !db::claim_pending_share_invite(&pool, &token, &other.email, other.id)
            .await
            .expect("wrong address")
    );

    let friend = register(&pool, &email).await;
    assert!(
        db::claim_pending_share_invite(&pool, &token, &email, friend.id)
            .await
            .expect("claim")
    );
    assert_eq!(
        db::list_share_invites(&pool, friend.id)
            .await
            .expect("inbox")
            .len(),
        1
    );

    assert!(
        !db::claim_pending_share_invite(&pool, &token, &email, other.id)
            .await
            .expect("second use")
    );
}

#[tokio::test]
async fn inviting_again_replaces_the_claim_link() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    let first = pending_claim(&pool, &owner, cat, &email).await;
    let second = pending_claim(&pool, &owner, cat, &email).await;
    assert_ne!(first, second);

    let friend = register(&pool, &email).await;
    assert!(
        !db::claim_pending_share_invite(&pool, &first, &email, friend.id)
            .await
            .expect("old link")
    );
    assert!(
        db::claim_pending_share_invite(&pool, &second, &email, friend.id)
            .await
            .expect("new link")
    );
}

#[tokio::test]
async fn inviting_again_reaches_someone_who_registered_without_the_link_once_verified() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    pending_claim(&pool, &owner, cat, &email).await;
    let friend = register(&pool, &email).await;

    // Registering proves nothing about the address, so the invite keeps waiting.
    invite(&pool, &owner, cat, &email)
        .await
        .expect("invite again");
    assert!(db::list_share_invites(&pool, friend.id)
        .await
        .expect("inbox")
        .is_empty());

    db::mark_email_verified(&pool, friend.id)
        .await
        .expect("verify email");
    invite(&pool, &owner, cat, &email)
        .await
        .expect("invite after verifying");
    assert_eq!(
        db::list_share_invites(&pool, friend.id)
            .await
            .expect("inbox")
            .len(),
        1
    );
}

#[tokio::test]
async fn padded_email_cannot_take_an_existing_users_invite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let bob = user(&pool, "bob").await;
    invite(&pool, &owner, cat, &bob.email)
        .await
        .expect("invite");

    let padded = format!("{} ", bob.email);
    let duplicate = db::create_user(&pool, &padded, Some("password-long"), None, None, None).await;
    assert!(matches!(duplicate, Err(Error::Conflict(_))));

    let thief = user(&pool, "thief").await;
    let linked = db::link_pending_share_invites(&pool, &bob.email, thief.id)
        .await
        .expect("link");
    assert_eq!(linked, 0);
    assert_eq!(
        db::list_share_invites(&pool, bob.id)
            .await
            .expect("inbox")
            .len(),
        1
    );
}

/// An invite to a new address, with the owner's link and the link only its email carries.
async fn pending_claim_with_email_link(
    pool: &PgPool,
    owner: &User,
    category_id: Uuid,
    email: &str,
) -> (String, String) {
    let invite = match invite(pool, owner, category_id, email)
        .await
        .expect("invite")
    {
        CreateCategoryShareOutcome::Pending { invite } => invite,
        other => panic!("expected a pending invite, got {other:?}"),
    };
    let owner_token = invite.claim_token.expect("owner link");
    let email_token = db::issue_email_claim_token(pool, invite.id, &owner_token)
        .await
        .expect("issue")
        .expect("email link");
    assert_ne!(owner_token, email_token);
    (owner_token, email_token)
}

async fn is_verified(pool: &PgPool, user_id: Uuid) -> bool {
    db::find_user_by_id(pool, user_id)
        .await
        .expect("user")
        .expect("exists")
        .email_verified_at
        .is_some()
}

#[tokio::test]
async fn emailed_link_claims_the_invite_and_verifies_the_address() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    let (owner_token, email_token) =
        pending_claim_with_email_link(&pool, &owner, cat, &email).await;

    let friend = register(&pool, &email).await;
    assert!(!is_verified(&pool, friend.id).await);
    assert!(
        db::claim_pending_share_invite(&pool, &email_token, &email, friend.id)
            .await
            .expect("claim")
    );
    assert!(is_verified(&pool, friend.id).await);
    assert_eq!(
        db::list_share_invites(&pool, friend.id)
            .await
            .expect("inbox")
            .len(),
        1
    );

    // Both links die with the claim.
    let other = register(&pool, &format!("other-{}@invites.test", Uuid::new_v4())).await;
    for token in [&email_token, &owner_token] {
        assert!(
            !db::claim_pending_share_invite(&pool, token, &email, other.id)
                .await
                .expect("second use")
        );
    }
}

#[tokio::test]
async fn owners_link_claims_without_verifying_and_voids_the_emailed_link() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    let (owner_token, email_token) =
        pending_claim_with_email_link(&pool, &owner, cat, &email).await;

    let friend = register(&pool, &email).await;
    assert!(
        db::claim_pending_share_invite(&pool, &owner_token, &email, friend.id)
            .await
            .expect("claim")
    );
    assert!(!is_verified(&pool, friend.id).await);
    assert!(
        !db::claim_pending_share_invite(&pool, &email_token, &email, friend.id)
            .await
            .expect("emailed link after the owner's")
    );
    assert!(!is_verified(&pool, friend.id).await);
}

#[tokio::test]
async fn inviting_again_voids_the_emailed_link() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    let (first_owner, first_email) =
        pending_claim_with_email_link(&pool, &owner, cat, &email).await;
    let second_owner = pending_claim(&pool, &owner, cat, &email).await;

    // The old owner's link can no longer be paired with a new emailed link.
    let pending =
        db::list_pending_for_subject(&pool, owner.id, PendingShareSubjectType::Category, cat)
            .await
            .expect("pending");
    assert!(
        db::issue_email_claim_token(&pool, pending[0].id, &first_owner)
            .await
            .expect("issue")
            .is_none()
    );

    let friend = register(&pool, &email).await;
    assert!(
        !db::claim_pending_share_invite(&pool, &first_email, &email, friend.id)
            .await
            .expect("old emailed link")
    );
    assert!(!is_verified(&pool, friend.id).await);
    assert!(
        db::claim_pending_share_invite(&pool, &second_owner, &email, friend.id)
            .await
            .expect("new link")
    );
}

#[tokio::test]
async fn emailed_link_cannot_claim_or_verify_another_address() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let cat = category(&pool, &owner).await;
    let email = format!("friend-{}@invites.test", Uuid::new_v4());
    let (_, email_token) = pending_claim_with_email_link(&pool, &owner, cat, &email).await;

    let other_email = format!("other-{}@invites.test", Uuid::new_v4());
    let other = register(&pool, &other_email).await;
    assert!(
        !db::claim_pending_share_invite(&pool, &email_token, &other_email, other.id)
            .await
            .expect("wrong address")
    );
    assert!(
        !db::claim_pending_share_invite_signed_in(&pool, &email_token, other.id)
            .await
            .expect("wrong address signed in")
    );
    assert!(!is_verified(&pool, other.id).await);

    // Still good for the invited address, signed in.
    let friend = register(&pool, &email).await;
    assert!(
        db::claim_pending_share_invite_signed_in(&pool, &email_token, friend.id)
            .await
            .expect("claim signed in")
    );
    assert!(is_verified(&pool, friend.id).await);
}
