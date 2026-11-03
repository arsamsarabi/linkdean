//! Email verification: links, purpose separation, invites held for unverified accounts, email
//! change and admin controls.
//! Requires `DATABASE_URL`. Skips when unset.

use chrono::Duration;
use linkdean_core::{
    CreateCategory, CreateCategoryShare, CreateCategoryShareOutcome, CreateDashboard,
    CreateDashboardShare, CreateDashboardShareOutcome, Error, ShareAccess, UpdateProfile, User,
    VaultKeyring,
};
use linkdean_db::{self as db, TokenPurpose};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping email verification tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn address(prefix: &str) -> String {
    format!("{prefix}-{}@verify.test", Uuid::new_v4())
}

/// A new, unverified password account.
async fn unverified(pool: &PgPool, prefix: &str) -> User {
    db::create_user(
        pool,
        &address(prefix),
        Some("password-long"),
        None,
        None,
        None,
    )
    .await
    .expect("user")
}

async fn verified(pool: &PgPool, prefix: &str) -> User {
    let u = unverified(pool, prefix).await;
    db::mark_email_verified(pool, u.id).await.expect("verify");
    reload(pool, u.id).await
}

async fn reload(pool: &PgPool, id: Uuid) -> User {
    db::find_user_by_id(pool, id)
        .await
        .expect("find")
        .expect("exists")
}

async fn verify_token(pool: &PgPool, uid: Uuid) -> String {
    db::create_account_token(
        pool,
        uid,
        TokenPurpose::VerifyEmail,
        Duration::hours(24),
        None,
        None,
    )
    .await
    .expect("token")
}

async fn category(pool: &PgPool, owner: &User) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Verify-{}", Uuid::new_v4()),
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

async fn share(pool: &PgPool, owner: &User, cat: Uuid, email: &str) -> CreateCategoryShareOutcome {
    db::create_share(
        pool,
        owner.id,
        cat,
        CreateCategoryShare {
            email: email.to_string(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("share")
}

async fn inbox_len(pool: &PgPool, uid: Uuid) -> usize {
    db::list_share_invites(pool, uid)
        .await
        .expect("inbox")
        .len()
}

fn is_conflict<T: std::fmt::Debug>(r: &linkdean_core::Result<T>) -> bool {
    matches!(r, Err(Error::Conflict(_)))
}

fn is_bad_request<T: std::fmt::Debug>(r: &linkdean_core::Result<T>) -> bool {
    matches!(r, Err(Error::BadRequest(_)))
}

#[tokio::test]
async fn a_new_account_is_unverified_and_a_link_verifies_it_once() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = unverified(&pool, "new").await;
    assert!(u.email_verified_at.is_none());
    assert!(!db::to_public(&pool, &u).await.unwrap().email_verified);

    let token = verify_token(&pool, u.id).await;
    assert_eq!(db::verify_email_token(&pool, &token).await.unwrap(), u.id);
    let u = reload(&pool, u.id).await;
    assert!(u.email_verified_at.is_some());
    assert!(db::to_public(&pool, &u).await.unwrap().email_verified);

    assert!(is_bad_request(&db::verify_email_token(&pool, &token).await));
    assert!(is_bad_request(
        &db::verify_email_token(&pool, "not-a-token").await
    ));
}

#[tokio::test]
async fn an_expired_verification_link_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = unverified(&pool, "expired").await;
    let token = verify_token(&pool, u.id).await;
    sqlx::query(
        "UPDATE account_tokens SET expires_at = now() - interval '1 second' WHERE user_id = $1",
    )
    .bind(u.id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(is_bad_request(&db::verify_email_token(&pool, &token).await));
    assert!(reload(&pool, u.id).await.email_verified_at.is_none());
}

#[tokio::test]
async fn a_new_verification_link_voids_the_old_one_but_not_a_reset_link() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = unverified(&pool, "newest").await;
    let reset = db::create_reset_token(&pool, u.id, Duration::minutes(30), None)
        .await
        .unwrap();
    let old = verify_token(&pool, u.id).await;
    let new = verify_token(&pool, u.id).await;
    assert!(is_bad_request(&db::verify_email_token(&pool, &old).await));
    db::verify_email_token(&pool, &new).await.unwrap();
    // One live token per purpose: the verification links left the reset link alone.
    db::consume_reset_token(&pool, &reset, "new-password-123", false)
        .await
        .unwrap();
}

#[tokio::test]
async fn tokens_only_work_for_their_purpose() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = unverified(&pool, "purpose").await;
    let reset = db::create_reset_token(&pool, u.id, Duration::minutes(30), None)
        .await
        .unwrap();
    assert!(is_bad_request(&db::verify_email_token(&pool, &reset).await));
    assert!(is_bad_request(
        &db::confirm_email_change(&pool, &reset).await
    ));
    assert!(reload(&pool, u.id).await.email_verified_at.is_none());

    let verify = verify_token(&pool, u.id).await;
    assert!(is_bad_request(
        &db::consume_reset_token(&pool, &verify, "new-password-123", false).await
    ));
    // Both still work for what they were made for.
    db::verify_email_token(&pool, &verify).await.unwrap();
    db::consume_reset_token(&pool, &reset, "new-password-123", false)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_share_to_an_unverified_account_waits_until_it_verifies() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = verified(&pool, "owner").await;
    let invitee = unverified(&pool, "invitee").await;
    let cat = category(&pool, &owner).await;

    match share(&pool, &owner, cat, &invitee.email).await {
        CreateCategoryShareOutcome::Pending { invite } => {
            assert!(invite.claim_token.is_some(), "address-only invite");
        }
        other => panic!("expected pending, got {other:?}"),
    }
    assert_eq!(inbox_len(&pool, invitee.id).await, 0);

    let token = verify_token(&pool, invitee.id).await;
    db::verify_email_token(&pool, &token).await.unwrap();
    assert_eq!(inbox_len(&pool, invitee.id).await, 1);
    // Attached, not granted: it still needs accepting.
    assert!(db::get_category(&pool, invitee.id, cat).await.is_err());
}

#[tokio::test]
async fn a_dashboard_share_to_an_unverified_account_waits_too() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = verified(&pool, "owner").await;
    let invitee = unverified(&pool, "invitee").await;
    let dash = db::create_dashboard(
        &pool,
        owner.id,
        CreateDashboard {
            title: Some("Shared".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard");

    let out = db::create_dashboard_share(
        &pool,
        owner.id,
        dash.id,
        CreateDashboardShare {
            email: invitee.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("share");
    match out {
        CreateDashboardShareOutcome::Pending { invite } => assert!(invite.claim_token.is_some()),
        other => panic!("expected pending, got {other:?}"),
    }
    assert_eq!(inbox_len(&pool, invitee.id).await, 0);

    assert_eq!(db::mark_email_verified(&pool, invitee.id).await.unwrap(), 1);
    assert_eq!(inbox_len(&pool, invitee.id).await, 1);
}

#[tokio::test]
async fn a_signed_in_account_can_claim_an_invite_to_its_address() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = verified(&pool, "owner").await;
    let invitee = unverified(&pool, "invitee").await;
    let other = verified(&pool, "other").await;
    let cat = category(&pool, &owner).await;
    let CreateCategoryShareOutcome::Pending { invite } =
        share(&pool, &owner, cat, &invitee.email).await
    else {
        panic!("expected pending");
    };
    let token = invite.claim_token.unwrap();

    assert!(
        !db::claim_pending_share_invite_signed_in(&pool, &token, other.id)
            .await
            .unwrap()
    );
    assert!(
        db::claim_pending_share_invite_signed_in(&pool, &token, invitee.id)
            .await
            .unwrap()
    );
    assert_eq!(inbox_len(&pool, invitee.id).await, 1);
}

#[tokio::test]
async fn email_change_applies_only_when_the_link_is_opened() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = unverified(&pool, "mover").await;
    let old_email = u.email.clone();
    let new_email = address("moved");

    assert!(matches!(
        db::start_email_change(&pool, u.id, "wrong-password", &new_email).await,
        Err(Error::Unauthorized)
    ));
    assert!(is_bad_request(
        &db::start_email_change(&pool, u.id, "password-long", &old_email).await
    ));
    assert!(is_bad_request(
        &db::start_email_change(&pool, u.id, "password-long", "no-at-sign").await
    ));

    // Mixed case and padding are normalized.
    let token = db::start_email_change(
        &pool,
        u.id,
        "password-long",
        &format!("  {}  ", new_email.to_uppercase()),
    )
    .await
    .unwrap();
    assert_eq!(reload(&pool, u.id).await.email, old_email);

    let (uid, old, new) = db::confirm_email_change(&pool, &token).await.unwrap();
    assert_eq!(
        (uid, old.as_str(), new.as_str()),
        (u.id, old_email.as_str(), new_email.as_str())
    );
    let after = reload(&pool, u.id).await;
    assert_eq!(after.email, new_email);
    assert!(after.email_verified_at.is_some());

    assert!(is_bad_request(
        &db::confirm_email_change(&pool, &token).await
    ));
}

#[tokio::test]
async fn a_newer_email_change_voids_the_older_one() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = verified(&pool, "mover").await;
    let first = db::start_email_change(&pool, u.id, "password-long", &address("first"))
        .await
        .unwrap();
    let second_email = address("second");
    let second = db::start_email_change(&pool, u.id, "password-long", &second_email)
        .await
        .unwrap();
    assert!(is_bad_request(
        &db::confirm_email_change(&pool, &first).await
    ));
    db::confirm_email_change(&pool, &second).await.unwrap();
    assert_eq!(reload(&pool, u.id).await.email, second_email);
}

#[tokio::test]
async fn email_change_refuses_sso_accounts_and_taken_addresses() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let sso = db::create_user(
        &pool,
        &address("sso"),
        None,
        None,
        Some(&Uuid::new_v4().to_string()),
        Some("https://idp.verify.test"),
    )
    .await
    .unwrap();
    assert!(is_bad_request(
        &db::start_email_change(&pool, sso.id, "anything-long", &address("x")).await
    ));

    let u = verified(&pool, "mover").await;
    let taken = unverified(&pool, "taken").await;
    assert!(is_conflict(
        &db::start_email_change(&pool, u.id, "password-long", &taken.email).await
    ));

    // Someone takes the address after the link went out.
    let target = address("target");
    let token = db::start_email_change(&pool, u.id, "password-long", &target)
        .await
        .unwrap();
    db::create_user(&pool, &target, Some("password-long"), None, None, None)
        .await
        .unwrap();
    assert!(is_conflict(&db::confirm_email_change(&pool, &token).await));
    assert_eq!(reload(&pool, u.id).await.email, u.email);
}

#[tokio::test]
async fn confirming_an_email_change_attaches_invites_for_the_new_address() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = verified(&pool, "owner").await;
    let u = verified(&pool, "mover").await;
    let cat = category(&pool, &owner).await;
    let target = address("target");
    share(&pool, &owner, cat, &target).await;

    let token = db::start_email_change(&pool, u.id, "password-long", &target)
        .await
        .unwrap();
    assert_eq!(inbox_len(&pool, u.id).await, 0);
    db::confirm_email_change(&pool, &token).await.unwrap();
    assert_eq!(inbox_len(&pool, u.id).await, 1);
}

#[tokio::test]
async fn an_email_change_voids_links_sent_to_the_old_address() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = unverified(&pool, "mover").await;
    let reset = db::create_reset_token(&pool, u.id, Duration::minutes(30), None)
        .await
        .unwrap();
    let verify = verify_token(&pool, u.id).await;
    let token = db::start_email_change(&pool, u.id, "password-long", &address("new"))
        .await
        .unwrap();
    db::confirm_email_change(&pool, &token).await.unwrap();
    assert!(is_bad_request(
        &db::consume_reset_token(&pool, &reset, "new-password-123", false).await
    ));
    assert!(is_bad_request(
        &db::verify_email_token(&pool, &verify).await
    ));
}

#[tokio::test]
async fn admin_can_remove_verification_and_change_the_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = verified(&pool, "owner").await;
    let u = verified(&pool, "target").await;
    let cat = category(&pool, &owner).await;
    let summary = |users: Vec<linkdean_core::AdminUserSummary>, id: Uuid| {
        users.into_iter().find(|s| s.id == id).expect("listed")
    };
    assert!(summary(db::list_admin_users(&pool).await.unwrap(), u.id).email_verified);

    db::set_email_verified(&pool, u.id, false).await.unwrap();
    assert!(!summary(db::list_admin_users(&pool).await.unwrap(), u.id).email_verified);
    match share(&pool, &owner, cat, &u.email).await {
        CreateCategoryShareOutcome::Pending { invite } => assert!(invite.claim_token.is_some()),
        other => panic!("expected pending, got {other:?}"),
    }
    assert_eq!(inbox_len(&pool, u.id).await, 0);

    db::set_email_verified(&pool, u.id, true).await.unwrap();
    assert_eq!(inbox_len(&pool, u.id).await, 1);

    let reset = db::create_reset_token(&pool, u.id, Duration::minutes(30), None)
        .await
        .unwrap();
    db::set_email_verified(&pool, u.id, false).await.unwrap();
    let new_email = address("admin-set");
    db::admin_change_email(&pool, u.id, &new_email)
        .await
        .unwrap();
    let after = reload(&pool, u.id).await;
    assert_eq!(after.email, new_email);
    assert!(after.email_verified_at.is_some());
    assert!(is_bad_request(
        &db::consume_reset_token(&pool, &reset, "new-password-123", false).await
    ));

    assert!(is_conflict(
        &db::admin_change_email(&pool, u.id, &owner.email).await
    ));
    assert!(matches!(
        db::set_email_verified(&pool, Uuid::new_v4(), true).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn racing_email_changes_to_one_address_give_one_conflict() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = verified(&pool, "a").await;
    let b = verified(&pool, "b").await;
    let target = address("race");
    let ta = db::start_email_change(&pool, a.id, "password-long", &target)
        .await
        .unwrap();
    let tb = db::start_email_change(&pool, b.id, "password-long", &target)
        .await
        .unwrap();
    let (ra, rb) = tokio::join!(
        db::confirm_email_change(&pool, &ta),
        db::confirm_email_change(&pool, &tb)
    );
    assert_eq!(
        [ra.is_ok(), rb.is_ok()].iter().filter(|ok| **ok).count(),
        1,
        "{ra:?} {rb:?}"
    );
    assert!(is_conflict(&ra) || is_conflict(&rb));
}

#[tokio::test]
async fn update_profile_no_longer_changes_the_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = verified(&pool, "profile").await;
    let patch = |email: &str| UpdateProfile {
        display_name: None,
        bio: None,
        timezone: None,
        email: Some(email.to_string()),
        profile_visibility: None,
    };
    assert!(is_bad_request(
        &db::update_profile(&pool, u.id, &patch(&address("other"))).await
    ));
    assert_eq!(reload(&pool, u.id).await.email, u.email);
    // Sending the current address back is accepted.
    db::update_profile(&pool, u.id, &patch(&u.email.to_uppercase()))
        .await
        .unwrap();
}

#[tokio::test]
async fn the_hard_gate_setting_round_trips() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let before = db::get_platform_settings(&pool).await.unwrap();
    let on = db::set_require_verified_email(&pool, true).await.unwrap();
    assert!(on.require_verified_email);
    assert!(
        db::get_platform_settings(&pool)
            .await
            .unwrap()
            .require_verified_email
    );
    db::set_require_verified_email(&pool, before.require_verified_email)
        .await
        .unwrap();
}
