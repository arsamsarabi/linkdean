//! Organization invite links (Phase B). Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    AcceptOrganizationInviteLink, CreateOrganization, CreateOrganizationInviteLink, CreateTeam,
    Error, OrganizationInviteTeamSpec, OrganizationRole, PatchOrganizationInviteLinks,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping organization_invite_links tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@orglink.test", Uuid::new_v4())
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

async fn unverified_user(pool: &sqlx::PgPool, prefix: &str) -> linkdean_core::User {
    db::create_user(
        pool,
        &unique_email(prefix),
        Some("password-long-enough"),
        Some(prefix),
        None,
        None,
    )
    .await
    .expect("user")
}

#[tokio::test]
async fn invite_links_off_by_default_and_create_refuses() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "off-owner").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Off Links".into(),
        },
    )
    .await
    .expect("org");
    assert!(!org.invite_links_enabled);

    let err = db::create_organization_invite_link(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInviteLink {
            role: OrganizationRole::Member,
            expires_in_days: Some(7),
            max_uses: Some(5),
            teams: vec![],
        },
    )
    .await
    .expect_err("create while disabled");
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn enable_create_accept_and_use_limit() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "lim-owner").await;
    let a = user(&pool, "lim-a").await;
    let b = user(&pool, "lim-b").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Limit Org".into(),
        },
    )
    .await
    .expect("org");

    let enabled = db::set_organization_invite_links_enabled(
        &pool,
        owner.id,
        org.id,
        PatchOrganizationInviteLinks {
            invite_links_enabled: true,
        },
    )
    .await
    .expect("enable");
    assert!(enabled.invite_links_enabled);

    let created = db::create_organization_invite_link(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInviteLink {
            role: OrganizationRole::Member,
            expires_in_days: Some(3),
            max_uses: Some(1),
            teams: vec![],
        },
    )
    .await
    .expect("create");
    assert!(created.code.starts_with("ldoi_"));
    assert_eq!(created.link.max_uses, Some(1));
    assert_eq!(created.link.use_count, 0);
    assert_eq!(created.link.status, "pending");

    db::accept_organization_invite_link(
        &pool,
        &a,
        AcceptOrganizationInviteLink {
            code: created.code.clone(),
        },
    )
    .await
    .expect("accept a");
    assert_eq!(
        db::membership_role(&pool, a.id, org.id)
            .await
            .expect("role"),
        Some(OrganizationRole::Member)
    );

    let err = db::accept_organization_invite_link(
        &pool,
        &b,
        AcceptOrganizationInviteLink {
            code: created.code.clone(),
        },
    )
    .await
    .expect_err("exhausted");
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn unverified_cannot_accept_and_revoke_blocks() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "uv-owner").await;
    let unverified = unverified_user(&pool, "uv-peer").await;
    let peer = user(&pool, "uv-ok").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Verify Org".into(),
        },
    )
    .await
    .expect("org");
    db::set_organization_invite_links_enabled(
        &pool,
        owner.id,
        org.id,
        PatchOrganizationInviteLinks {
            invite_links_enabled: true,
        },
    )
    .await
    .expect("enable");

    let created = db::create_organization_invite_link(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInviteLink {
            role: OrganizationRole::Member,
            expires_in_days: None,
            max_uses: None,
            teams: vec![],
        },
    )
    .await
    .expect("create");

    let err = db::accept_organization_invite_link(
        &pool,
        &unverified,
        AcceptOrganizationInviteLink {
            code: created.code.clone(),
        },
    )
    .await
    .expect_err("unverified");
    assert!(matches!(err, Error::EmailUnverified));

    db::revoke_organization_invite_link(&pool, owner.id, org.id, created.link.id)
        .await
        .expect("revoke");

    let err = db::accept_organization_invite_link(
        &pool,
        &peer,
        AcceptOrganizationInviteLink {
            code: created.code.clone(),
        },
    )
    .await
    .expect_err("revoked");
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn guest_link_requires_team_and_joins_only_that_team() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "gl-owner").await;
    let guest = user(&pool, "gl-guest").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Guest Link Org".into(),
        },
    )
    .await
    .expect("org");
    let team = db::create_team(
        &pool,
        owner.id,
        org.id,
        CreateTeam {
            name: "Only Team".into(),
        },
    )
    .await
    .expect("team");
    db::set_organization_invite_links_enabled(
        &pool,
        owner.id,
        org.id,
        PatchOrganizationInviteLinks {
            invite_links_enabled: true,
        },
    )
    .await
    .expect("enable");

    let err = db::create_organization_invite_link(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInviteLink {
            role: OrganizationRole::Guest,
            expires_in_days: Some(7),
            max_uses: Some(2),
            teams: vec![],
        },
    )
    .await
    .expect_err("guest needs team");
    assert!(matches!(err, Error::BadRequest(_)));

    let created = db::create_organization_invite_link(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInviteLink {
            role: OrganizationRole::Guest,
            expires_in_days: Some(7),
            max_uses: Some(2),
            teams: vec![OrganizationInviteTeamSpec {
                team_id: team.id,
                role: None,
            }],
        },
    )
    .await
    .expect("create guest link");

    db::accept_organization_invite_link(
        &pool,
        &guest,
        AcceptOrganizationInviteLink { code: created.code },
    )
    .await
    .expect("accept");

    assert_eq!(
        db::membership_role(&pool, guest.id, org.id)
            .await
            .expect("role"),
        Some(OrganizationRole::Guest)
    );
    let teams = db::list_teams_for_member(&pool, guest.id, org.id)
        .await
        .expect("teams");
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0].id, team.id);
}

#[tokio::test]
async fn already_member_and_disabled_refuse_accept() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "am-owner").await;
    let peer = user(&pool, "am-peer").await;
    let org = db::create_organization(
        &pool,
        &owner,
        CreateOrganization {
            name: "Already Org".into(),
        },
    )
    .await
    .expect("org");
    db::set_organization_invite_links_enabled(
        &pool,
        owner.id,
        org.id,
        PatchOrganizationInviteLinks {
            invite_links_enabled: true,
        },
    )
    .await
    .expect("enable");
    let created = db::create_organization_invite_link(
        &pool,
        &owner,
        org.id,
        CreateOrganizationInviteLink {
            role: OrganizationRole::Member,
            expires_in_days: Some(7),
            max_uses: None,
            teams: vec![],
        },
    )
    .await
    .expect("create");

    db::accept_organization_invite_link(
        &pool,
        &peer,
        AcceptOrganizationInviteLink {
            code: created.code.clone(),
        },
    )
    .await
    .expect("first accept");

    let err = db::accept_organization_invite_link(
        &pool,
        &peer,
        AcceptOrganizationInviteLink {
            code: created.code.clone(),
        },
    )
    .await
    .expect_err("already member");
    assert!(matches!(err, Error::Conflict(_)));

    db::set_organization_invite_links_enabled(
        &pool,
        owner.id,
        org.id,
        PatchOrganizationInviteLinks {
            invite_links_enabled: false,
        },
    )
    .await
    .expect("disable");

    let outsider = user(&pool, "am-out").await;
    let err = db::accept_organization_invite_link(
        &pool,
        &outsider,
        AcceptOrganizationInviteLink { code: created.code },
    )
    .await
    .expect_err("disabled");
    assert!(matches!(err, Error::BadRequest(_)));
}
