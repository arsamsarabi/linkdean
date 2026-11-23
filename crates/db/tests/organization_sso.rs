//! Organization SSO config (Phase D1). Requires Postgres (`DATABASE_URL`).

use linkdean_core::crypto::encrypt_org_sso_secret;
use linkdean_core::{CreateOrganization, CreateOrganizationDomain, Error, OrganizationRole};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping organization_sso tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@orgsso.test", Uuid::new_v4())
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

async fn verified_domain(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    org_id: Uuid,
    domain: &str,
) -> linkdean_core::OrganizationDomainView {
    let created = db::create_organization_domain(
        pool,
        owner_id,
        org_id,
        CreateOrganizationDomain {
            domain: domain.into(),
        },
    )
    .await
    .expect("domain");
    db::verify_organization_domain_with_txt(
        pool,
        owner_id,
        org_id,
        created.id,
        &[created.txt_value.clone()],
    )
    .await
    .expect("verify")
}

#[tokio::test]
async fn upsert_requires_secret_on_create_and_verified_domain_to_enable() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "sso-owner").await;
    let org = make_org(&pool, &owner, "SSO Co").await;
    let redirect = "http://localhost/api/auth/org-oidc/callback";
    let (ct, nonce) = encrypt_org_sso_secret("test-session-secret!!", "client-secret").unwrap();

    let err = db::upsert_organization_sso(
        &pool,
        owner.id,
        org.id,
        "https://idp.example/realms/acme",
        "client-id",
        None,
        "Company SSO",
        false,
        redirect,
    )
    .await
    .expect_err("secret required");
    assert!(matches!(err, Error::BadRequest(_)));

    let configured = db::upsert_organization_sso(
        &pool,
        owner.id,
        org.id,
        "https://idp.example/realms/acme",
        "client-id",
        Some((ct.clone(), nonce.clone())),
        "Acme SSO",
        false,
        redirect,
    )
    .await
    .expect("configure disabled");
    assert!(!configured.enabled);
    assert_eq!(configured.display_name, "Acme SSO");

    let enable_err = db::upsert_organization_sso(
        &pool,
        owner.id,
        org.id,
        "https://idp.example/realms/acme",
        "client-id",
        None,
        "Acme SSO",
        true,
        redirect,
    )
    .await
    .expect_err("need verified domain");
    assert!(matches!(enable_err, Error::BadRequest(_)));

    let domain = format!("sso-{}.example", Uuid::new_v4());
    verified_domain(&pool, owner.id, org.id, &domain).await;

    let enabled = db::upsert_organization_sso(
        &pool,
        owner.id,
        org.id,
        "https://idp.example/realms/acme",
        "client-id",
        None,
        "Acme SSO",
        true,
        redirect,
    )
    .await
    .expect("enable");
    assert!(enabled.enabled);

    let discovered = db::discover_organization_sso(&pool, &format!("alice@{domain}"))
        .await
        .expect("discover");
    assert!(discovered.sso);
    assert_eq!(discovered.organization_id, Some(org.id));

    let no_match = db::discover_organization_sso(&pool, "bob@other.example")
        .await
        .expect("discover miss");
    assert!(!no_match.sso);

    assert!(
        db::email_matches_org_verified_domain(&pool, org.id, &format!("alice@{domain}"))
            .await
            .expect("match")
    );

    db::delete_organization_sso(&pool, owner.id, org.id)
        .await
        .expect("delete");
    let gone = db::get_organization_sso(&pool, owner.id, org.id, redirect)
        .await
        .expect("get");
    assert!(gone.is_none());
}

#[tokio::test]
async fn member_cannot_configure_sso() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "sso-owner2").await;
    let member = user(&pool, "sso-member").await;
    let org = make_org(&pool, &owner, "SSO Gate").await;
    db::add_organization_member(&pool, org.id, member.id, OrganizationRole::Member)
        .await
        .expect("add member");
    let (ct, nonce) = encrypt_org_sso_secret("test-session-secret!!", "secret").unwrap();
    let err = db::upsert_organization_sso(
        &pool,
        member.id,
        org.id,
        "https://idp.example/realms/x",
        "cid",
        Some((ct, nonce)),
        "SSO",
        false,
        "http://localhost/callback",
    )
    .await
    .expect_err("forbidden");
    assert!(matches!(err, Error::Forbidden));
}
