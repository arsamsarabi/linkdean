//! Filing and resolving abuse reports.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    AbuseAction, AbuseTargetType, CreateAbuseReport, CreateCategory, Error, ResolveAbuseReport,
    UpdateAdminUser, User,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping abuse report tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    db::create_user(
        pool,
        &format!("{prefix}-{}@abuse.test", Uuid::new_v4()),
        Some("account-password"),
        None,
        None,
        None,
    )
    .await
    .expect("user")
}

async fn admin(pool: &PgPool) -> User {
    let u = user(pool, "admin").await;
    db::update_admin_user(
        pool,
        u.id,
        UpdateAdminUser {
            api_keys_enabled: None,
            is_admin: Some(true),
        },
    )
    .await
    .expect("promote");
    u
}

fn report(kind: AbuseTargetType, target_id: &str) -> CreateAbuseReport {
    CreateAbuseReport {
        target_type: kind,
        target_id: target_id.into(),
        reason: "spam".into(),
        details: None,
    }
}

fn resolve(action: AbuseAction, note: Option<&str>) -> ResolveAbuseReport {
    ResolveAbuseReport {
        action,
        note: note.map(str::to_string),
    }
}

async fn public_list(pool: &PgPool, owner: &User) -> (Uuid, String) {
    let cat = db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Abuse-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category");
    let link = db::enable_category_public_link(pool, owner.id, cat.id)
        .await
        .expect("publish");
    let slug = link.slug.clone().expect("slug");
    (cat.id, slug)
}

#[tokio::test]
async fn reports_reject_bad_input_and_unreportable_targets() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let reporter = user(&pool, "reporter").await;
    let (cat_id, slug) = public_list(&pool, &owner).await;

    let mut bad_reason = report(AbuseTargetType::PublicList, &slug);
    bad_reason.reason = "rude".into();
    assert!(matches!(
        db::create_report(&pool, reporter.id, &bad_reason).await,
        Err(Error::BadRequest(_))
    ));
    assert!(matches!(
        db::create_report(
            &pool,
            reporter.id,
            &report(AbuseTargetType::PublicList, "ldl_nope")
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::create_report(&pool, owner.id, &report(AbuseTargetType::PublicList, &slug)).await,
        Err(Error::BadRequest(_))
    ));
    // A hidden profile reads as not found, like any other denial.
    assert!(matches!(
        db::create_report(
            &pool,
            reporter.id,
            &report(AbuseTargetType::Profile, &owner.id.to_string())
        )
        .await,
        Err(Error::NotFound)
    ));

    // By slug or by category id, the second report returns the first while it is open.
    let first = db::create_report(
        &pool,
        reporter.id,
        &report(AbuseTargetType::PublicList, &slug),
    )
    .await
    .expect("by slug");
    let again = db::create_report(
        &pool,
        reporter.id,
        &report(AbuseTargetType::PublicList, &cat_id.to_string()),
    )
    .await
    .expect("by id");
    assert_eq!(first.id, again.id);
    assert_eq!(first.target_user_id, Some(owner.id));
}

#[tokio::test]
async fn hiding_a_reported_list_revokes_its_link_and_resolves_once() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let reporter = user(&pool, "reporter").await;
    let a = admin(&pool).await;
    let (cat_id, slug) = public_list(&pool, &owner).await;
    let filed = db::create_report(
        &pool,
        reporter.id,
        &report(AbuseTargetType::PublicList, &slug),
    )
    .await
    .expect("report");
    assert!(db::list_reports(&pool, "open")
        .await
        .expect("open")
        .iter()
        .any(|r| r.id == filed.id));

    let done = db::resolve_report(&pool, a.id, filed.id, &resolve(AbuseAction::Hide, None))
        .await
        .expect("hide");
    assert_eq!(done.status, "resolved");
    assert_eq!(done.resolution_action.as_deref(), Some("hide"));
    assert!(db::get_category_public_link(&pool, owner.id, cat_id)
        .await
        .expect("link")
        .slug
        .is_none());
    assert!(matches!(
        db::resolve_report(&pool, a.id, filed.id, &resolve(AbuseAction::Dismiss, None)).await,
        Err(Error::Conflict(_))
    ));
}

#[tokio::test]
async fn banning_from_a_report_bans_once_and_a_warning_needs_a_note() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "owner").await;
    let reporter = user(&pool, "reporter").await;
    let a = admin(&pool).await;
    let (_, slug) = public_list(&pool, &owner).await;
    let first = db::create_report(
        &pool,
        reporter.id,
        &report(AbuseTargetType::PublicList, &slug),
    )
    .await
    .expect("first");

    assert!(matches!(
        db::resolve_report(&pool, a.id, first.id, &resolve(AbuseAction::Warn, None)).await,
        Err(Error::BadRequest(_))
    ));
    db::resolve_report(
        &pool,
        a.id,
        first.id,
        &resolve(AbuseAction::Ban, Some("spam")),
    )
    .await
    .expect("ban");
    let ban = db::active_ban(&pool, owner.id)
        .await
        .expect("lookup")
        .expect("banned");
    assert_eq!(ban.reason, "spam");

    // A later report about the same user still resolves with ban.
    let (_, other_slug) = public_list(&pool, &owner).await;
    let second = db::create_report(
        &pool,
        reporter.id,
        &report(AbuseTargetType::PublicList, &other_slug),
    )
    .await
    .expect("second");
    let done = db::resolve_report(&pool, a.id, second.id, &resolve(AbuseAction::Ban, None))
        .await
        .expect("already banned still resolves");
    assert_eq!(done.status, "resolved");
}
