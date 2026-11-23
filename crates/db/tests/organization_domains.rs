//! Organization verified domains (Phase C1). Requires Postgres (`DATABASE_URL`).

use linkdean_core::{
    CreateOrganization, CreateOrganizationDomain, DomainJoinMode, Error, OrganizationRole,
    PatchOrganizationDomain,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping organization_domains tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@orgdomain.test", Uuid::new_v4())
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

async fn make_org(
    pool: &sqlx::PgPool,
    owner: &linkdean_core::User,
    name: &str,
) -> linkdean_core::OrganizationWorkspace {
    db::create_organization(pool, owner, CreateOrganization { name: name.into() })
        .await
        .expect("org")
}

#[tokio::test]
async fn admin_adds_lists_verifies_and_patches_join_mode() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "dom-owner").await;
    let org = make_org(&pool, &owner, "Domain Co").await;

    let created = db::create_organization_domain(
        &pool,
        owner.id,
        org.id,
        CreateOrganizationDomain {
            domain: format!("acme-{}.example", Uuid::new_v4()),
        },
    )
    .await
    .expect("create");
    assert!(created.verified_at.is_none());
    assert_eq!(created.join_mode, DomainJoinMode::AutoJoin);
    assert!(created.txt_name.starts_with("_linkdean-challenge."));
    assert!(created
        .txt_value
        .starts_with("linkdean-domain-verification="));

    let listed = db::list_organization_domains(&pool, owner.id, org.id)
        .await
        .expect("list");
    assert_eq!(listed.len(), 1);

    let bad = db::verify_organization_domain_with_txt(
        &pool,
        owner.id,
        org.id,
        created.id,
        &["wrong".into()],
    )
    .await
    .expect_err("mismatch");
    assert!(matches!(bad, Error::BadRequest(_)));

    let verified = db::verify_organization_domain_with_txt(
        &pool,
        owner.id,
        org.id,
        created.id,
        &[created.txt_value.clone()],
    )
    .await
    .expect("verify");
    assert!(verified.verified_at.is_some());

    let again = db::verify_organization_domain_with_txt(&pool, owner.id, org.id, created.id, &[])
        .await
        .expect("idempotent");
    assert!(again.verified_at.is_some());

    let patched = db::patch_organization_domain(
        &pool,
        owner.id,
        org.id,
        created.id,
        PatchOrganizationDomain {
            join_mode: DomainJoinMode::Request,
        },
    )
    .await
    .expect("patch");
    assert_eq!(patched.join_mode, DomainJoinMode::Request);

    db::delete_organization_domain(&pool, owner.id, org.id, created.id)
        .await
        .expect("delete");
    assert!(db::list_organization_domains(&pool, owner.id, org.id)
        .await
        .expect("list empty")
        .is_empty());
}

#[tokio::test]
async fn refuses_blocked_and_duplicate_domains() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "dom-block").await;
    let org = make_org(&pool, &owner, "Block Co").await;

    let err = db::create_organization_domain(
        &pool,
        owner.id,
        org.id,
        CreateOrganizationDomain {
            domain: "gmail.com".into(),
        },
    )
    .await
    .expect_err("gmail");
    assert!(matches!(err, Error::BadRequest(_)));

    let domain = format!("unique-{}.example", Uuid::new_v4());
    db::create_organization_domain(
        &pool,
        owner.id,
        org.id,
        CreateOrganizationDomain {
            domain: domain.clone(),
        },
    )
    .await
    .expect("first");

    let other = user(&pool, "dom-other").await;
    let org2 = make_org(&pool, &other, "Other Co").await;
    let dup = db::create_organization_domain(
        &pool,
        other.id,
        org2.id,
        CreateOrganizationDomain { domain },
    )
    .await
    .expect_err("duplicate");
    assert!(matches!(dup, Error::Conflict(_)));
}

#[tokio::test]
async fn member_cannot_manage_domains() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "dom-own2").await;
    let member = user(&pool, "dom-mem").await;
    let org = make_org(&pool, &owner, "Member Gate").await;
    db::add_organization_member(&pool, org.id, member.id, OrganizationRole::Member)
        .await
        .expect("add member");

    let err = db::create_organization_domain(
        &pool,
        member.id,
        org.id,
        CreateOrganizationDomain {
            domain: format!("nope-{}.example", Uuid::new_v4()),
        },
    )
    .await
    .expect_err("forbidden");
    assert!(matches!(err, Error::Forbidden));
}

#[tokio::test]
async fn auto_join_and_request_flow() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "join-own").await;
    let org = make_org(&pool, &owner, "Join Co").await;
    let domain = format!("join-{}.example", Uuid::new_v4());
    let created = db::create_organization_domain(
        &pool,
        owner.id,
        org.id,
        CreateOrganizationDomain {
            domain: domain.clone(),
        },
    )
    .await
    .expect("create");
    db::verify_organization_domain_with_txt(
        &pool,
        owner.id,
        org.id,
        created.id,
        &[created.txt_value.clone()],
    )
    .await
    .expect("verify");

    let joiner = db::create_user(
        &pool,
        &format!("person@{}", domain),
        Some("password-long-enough"),
        Some("joiner"),
        None,
        None,
    )
    .await
    .expect("joiner");
    db::mark_email_verified(&pool, joiner.id)
        .await
        .expect("verify email");

    let n = db::auto_join_organizations_for_verified_email(&pool, joiner.id)
        .await
        .expect("auto");
    assert_eq!(n, 1);
    assert!(db::membership_role(&pool, joiner.id, org.id)
        .await
        .expect("role")
        .is_some());

    // request mode
    let owner2 = user(&pool, "req-own").await;
    let org2 = make_org(&pool, &owner2, "Request Co").await;
    let domain2 = format!("req-{}.example", Uuid::new_v4());
    let d2 = db::create_organization_domain(
        &pool,
        owner2.id,
        org2.id,
        CreateOrganizationDomain {
            domain: domain2.clone(),
        },
    )
    .await
    .expect("d2");
    db::verify_organization_domain_with_txt(
        &pool,
        owner2.id,
        org2.id,
        d2.id,
        &[d2.txt_value.clone()],
    )
    .await
    .expect("v2");
    db::patch_organization_domain(
        &pool,
        owner2.id,
        org2.id,
        d2.id,
        PatchOrganizationDomain {
            join_mode: DomainJoinMode::Request,
        },
    )
    .await
    .expect("patch");

    let requester = db::create_user(
        &pool,
        &format!("ask@{}", domain2),
        Some("password-long-enough"),
        Some("ask"),
        None,
        None,
    )
    .await
    .expect("req");
    db::mark_email_verified(&pool, requester.id)
        .await
        .expect("ver");

    let outcome = db::request_or_join_via_domain(&pool, requester.id, org2.id)
        .await
        .expect("request");
    assert_eq!(outcome.status, "pending");

    let pending = db::list_organization_domain_join_requests(&pool, owner2.id, org2.id)
        .await
        .expect("list");
    assert_eq!(pending.len(), 1);
    db::approve_organization_domain_join_request(&pool, owner2.id, org2.id, pending[0].id)
        .await
        .expect("approve");
    assert!(db::membership_role(&pool, requester.id, org2.id)
        .await
        .expect("role2")
        .is_some());
}
