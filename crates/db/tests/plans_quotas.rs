//! Managed plan quotas. Requires Postgres (`DATABASE_URL`). Skips when unset.

use linkdean_core::{AccountPlan, Error};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping plans_quotas tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

#[tokio::test]
async fn free_plan_caps_vaults_at_one() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("plan-vault-{}@test.local", Uuid::new_v4());
    let user = db::create_user_gated(
        &pool,
        &email,
        Some("account-password"),
        None,
        None,
        None,
        db::SignupGate {
            plan: Some("free"),
            ..Default::default()
        },
    )
    .await
    .expect("user");
    assert_eq!(user.plan.as_deref(), Some("free"));

    let limits = db::resolve_limits_for(&pool, user.id)
        .await
        .expect("limits");
    assert_eq!(limits.max_vaults, Some(1));
    assert_eq!(limits.max_bookmarks, Some(5_000));

    db::create_vault(&pool, user.id, "Only", "passphrase-long")
        .await
        .expect("first vault");
    let err = db::refuse_vault_quota(&pool, user.id, &limits)
        .await
        .expect_err("second vault blocked");
    match err {
        Error::QuotaExceeded { meter, limit, used } => {
            assert_eq!(meter, db::METER_VAULTS);
            assert_eq!(limit, 1);
            assert_eq!(used, 1);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn self_host_null_plan_ignores_free_caps() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("plan-selfhost-{}@test.local", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user");
    // Sibling tests may call ensure_managed_free_plans; keep this account on self-host.
    sqlx::query("UPDATE users SET plan = NULL WHERE id = $1")
        .bind(user.id)
        .execute(&pool)
        .await
        .expect("clear plan");
    let limits = db::resolve_limits_for(&pool, user.id)
        .await
        .expect("limits");
    assert!(limits.max_bookmarks.is_none());
    assert!(limits.max_vaults.is_none());
}

#[tokio::test]
async fn ensure_managed_free_plans_backfills() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let email = format!("plan-backfill-{}@test.local", Uuid::new_v4());
    let user = db::create_user(&pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user");
    assert!(user.plan.is_none());
    let n = db::ensure_managed_free_plans(&pool)
        .await
        .expect("backfill");
    assert!(n >= 1);
    let plan = db::get_user_plan(&pool, user.id).await.expect("plan");
    assert_eq!(plan, Some(AccountPlan::Free));
}
