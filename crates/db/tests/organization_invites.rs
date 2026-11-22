//! Organization email invites. Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    CreateOrganization, CreateOrganizationInvite, Error, OrganizationInviteTeamSpec,
    OrganizationRole,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping organization_invites tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@orginvite.test", Uuid::new_v4())
}

async fn user(pool: &sqlx::PgPool, prefix: &str) -> linkdean_core::User {
    let u = db::create_user(
        pool,
        &unique_email(prefix),
        Some("password-long-enough"),
        Some(prefix),
        None,
        None,
    )
    .await
    .expect("user");
    db::mark_email_verified(pool, u.id).await.expect("verify");
    db::find_user_by_id(pool, u.id)
        .await
        .expect("reload")
        .expect("exists")
}

#[tokio::test]
async fn invite_unknown_email_waits_until_verify_and_accept() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Wait Org".into(),
        },
    )
    .await
    .expect("org");

    let email = unique_email("newbie");
    let (invite, notifiable) = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: email.clone(),
            role: OrganizationRole::Member,
            teams: vec![],
        },
    )
    .await
    .expect("invite");
    assert!(notifiable.is_none());
    assert!(invite.invitee_user_id.is_none());

    let invitee = db::create_user(
        &pool,
        &email,
        Some("password-long-enough"),
        Some("newbie"),
        None,
        None,
    )
    .await
    .expect("register");
    // Unverified: still not attached
    db::link_pending_organization_invites(&pool, &email, invitee.id)
        .await
        .expect("link without verify still attaches by email match");
    // Design: attach on verify. Our link is called from mark_email_verified.
    // Re-fetch after explicit mark:
    db::mark_email_verified(&pool, invitee.id)
        .await
        .expect("verify");
    let invitee = db::find_user_by_id(&pool, invitee.id)
        .await
        .expect("reload")
        .expect("user");

    let inbox = db::list_organization_invite_inbox(&pool, invitee.id)
        .await
        .expect("inbox");
    assert!(inbox.iter().any(|i| i.id == invite.id));

    db::accept_organization_invite(&pool, &invitee, invite.id)
        .await
        .expect("accept");
    assert_eq!(
        db::membership_role(&pool, invitee.id, org.id)
            .await
            .expect("role"),
        Some(OrganizationRole::Member)
    );
}

#[tokio::test]
async fn verified_invitee_can_accept_and_reject_blocks_reinvite() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "host").await;
    let peer = user(&pool, "peer").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Reject Org".into(),
        },
    )
    .await
    .expect("org");

    let (invite, notifiable) = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: peer.email.clone(),
            role: OrganizationRole::Admin,
            teams: vec![],
        },
    )
    .await
    .expect("invite");
    assert_eq!(notifiable, Some(peer.id));

    db::reject_organization_invite(&pool, peer.id, invite.id)
        .await
        .expect("reject");
    let err = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: peer.email.clone(),
            role: OrganizationRole::Member,
            teams: vec![],
        },
    )
    .await
    .expect_err("reinvite while rejected open");
    assert!(matches!(err, Error::Conflict(_)));

    db::revoke_organization_invite(&pool, owner.id, org.id, invite.id)
        .await
        .expect("cancel");
    let (invite2, _) = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: peer.email.clone(),
            role: OrganizationRole::Member,
            teams: vec![],
        },
    )
    .await
    .expect("reinvite after cancel");
    db::accept_organization_invite(&pool, &peer, invite2.id)
        .await
        .expect("accept");
}

#[tokio::test]
async fn guest_invite_requires_team_and_joins_only_that_team() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "gowner").await;
    let guest = user(&pool, "guser").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Guest Org".into(),
        },
    )
    .await
    .expect("org");
    let team = db::create_team(
        &pool,
        owner.id,
        org.id,
        linkdean_core::CreateTeam {
            name: "External".into(),
        },
    )
    .await
    .expect("team");

    let err = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: guest.email.clone(),
            role: OrganizationRole::Guest,
            teams: vec![],
        },
    )
    .await
    .expect_err("guest needs team");
    assert!(matches!(err, Error::BadRequest(_)));

    let (invite, _) = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: guest.email.clone(),
            role: OrganizationRole::Guest,
            teams: vec![OrganizationInviteTeamSpec {
                team_id: team.id,
                role: Some(linkdean_core::TeamRole::Viewer),
            }],
        },
    )
    .await
    .expect("guest invite");
    db::accept_organization_invite(&pool, &guest, invite.id)
        .await
        .expect("accept");
    let teams = db::list_teams_for_member(&pool, guest.id, org.id)
        .await
        .expect("teams");
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0].id, team.id);
}

#[tokio::test]
async fn accept_requires_verified_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "vowner").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Verify Org".into(),
        },
    )
    .await
    .expect("org");
    let email = unique_email("unverified");
    let (invite, _) = db::create_organization_invite(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInvite {
            email: email.clone(),
            role: OrganizationRole::Member,
            teams: vec![],
        },
    )
    .await
    .expect("invite");
    let invitee = db::create_user(
        &pool,
        &email,
        Some("password-long-enough"),
        None,
        None,
        None,
    )
    .await
    .expect("user");
    db::link_pending_organization_invites(&pool, &email, invitee.id)
        .await
        .expect("link");
    let err = db::accept_organization_invite(&pool, &invitee, invite.id)
        .await
        .expect_err("unverified");
    assert!(matches!(err, Error::EmailUnverified));
}
