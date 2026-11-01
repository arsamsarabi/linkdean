//! Account bans and moderation of what a banned user exposes.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use chrono::{Duration, Utc};
use linkdean_core::{
    ChangeBans, CreateBan, CreateBookmark, Error, LiftBans, UpdateAdminUser, User, VaultKeyring,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping moderation tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@moderation.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
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

fn ban(days: Option<i64>) -> CreateBan {
    CreateBan {
        reason: "spam".into(),
        ends_at: None,
        days,
    }
}

async fn expire_bans(pool: &PgPool, user_id: Uuid) {
    sqlx::query("UPDATE user_bans SET ends_at = now() - interval '1 hour' WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .expect("expire");
}

async fn demote_other_admins(pool: &PgPool, keep: &[Uuid]) {
    // Only one test in this file may touch admins, or it would race with itself.
    sqlx::query("UPDATE users SET is_admin = FALSE WHERE is_admin AND id <> ALL($1)")
        .bind(keep)
        .execute(pool)
        .await
        .expect("demote");
}

#[tokio::test]
async fn a_ban_is_active_until_it_expires_or_is_lifted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "actor").await;
    let target = user(&pool, "target").await;

    assert!(db::active_ban(&pool, target.id)
        .await
        .expect("none")
        .is_none());
    let made = db::ban_user(&pool, a.id, target.id, &ban(None))
        .await
        .expect("ban");
    assert!(made.ends_at.is_none(), "no duration means permanent");
    assert!(db::active_ban(&pool, target.id)
        .await
        .expect("on")
        .is_some());

    expire_bans(&pool, target.id).await;
    assert!(db::active_ban(&pool, target.id)
        .await
        .expect("off")
        .is_none());

    db::ban_user(&pool, a.id, target.id, &ban(Some(7)))
        .await
        .expect("a new ban after expiry");
    let lift = LiftBans {
        user_ids: vec![target.id],
    };
    db::lift_bans(&pool, a.id, &lift).await.expect("lift");
    assert!(db::active_ban(&pool, target.id)
        .await
        .expect("lifted")
        .is_none());
}

#[tokio::test]
async fn bans_are_refused_for_self_and_repeats() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "actor").await;
    let target = user(&pool, "target").await;

    let own = db::ban_user(&pool, a.id, a.id, &ban(None)).await;
    assert!(matches!(own, Err(Error::BadRequest(_))));

    db::ban_user(&pool, a.id, target.id, &ban(None))
        .await
        .expect("ban");
    let again = db::ban_user(&pool, a.id, target.id, &ban(None)).await;
    assert!(matches!(again, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn the_last_active_admin_cannot_be_banned_or_demoted() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let actor = user(&pool, "actor").await;
    let last = admin(&pool).await;
    demote_other_admins(&pool, &[last.id]).await;

    let banned = db::ban_user(&pool, actor.id, last.id, &ban(None)).await;
    assert!(matches!(banned, Err(Error::BadRequest(_))));

    let second = admin(&pool).await;
    db::ban_user(&pool, last.id, second.id, &ban(None))
        .await
        .expect("ban second");
    let demoted = db::update_admin_user(
        &pool,
        last.id,
        UpdateAdminUser {
            api_keys_enabled: None,
            is_admin: Some(false),
        },
    )
    .await;
    assert!(
        matches!(demoted, Err(Error::BadRequest(_))),
        "a banned admin does not count"
    );
}

#[tokio::test]
async fn durations_are_validated() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "actor").await;
    let target = user(&pool, "target").await;
    let past = CreateBan {
        reason: "spam".into(),
        ends_at: Some(Utc::now() - Duration::days(1)),
        days: None,
    };
    let both = CreateBan {
        reason: "spam".into(),
        ends_at: Some(Utc::now() + Duration::days(1)),
        days: Some(1),
    };
    let blank = CreateBan {
        reason: "   ".into(),
        ..CreateBan::default()
    };
    for req in [past, both, blank, ban(Some(0)), ban(Some(3651))] {
        let res = db::ban_user(&pool, a.id, target.id, &req).await;
        assert!(matches!(res, Err(Error::BadRequest(_))));
    }
    assert!(db::active_ban(&pool, target.id)
        .await
        .expect("none")
        .is_none());
}

#[tokio::test]
async fn bulk_changes_apply_to_all_or_none() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "actor").await;
    let one = user(&pool, "one").await;
    let two = user(&pool, "two").await;
    let clean = user(&pool, "clean").await;
    for u in [&one, &two] {
        db::ban_user(&pool, a.id, u.id, &ban(Some(1)))
            .await
            .expect("ban");
    }

    let bad = ChangeBans {
        user_ids: vec![one.id, clean.id],
        reason: Some("changed".into()),
        ..ChangeBans::default()
    };
    assert!(db::change_bans(&pool, a.id, &bad).await.is_err());
    let kept = db::active_ban(&pool, one.id)
        .await
        .expect("get")
        .expect("ban");
    assert_eq!(kept.reason, "spam");

    let good = ChangeBans {
        user_ids: vec![one.id, two.id],
        reason: Some("changed".into()),
        permanent: true,
        ..ChangeBans::default()
    };
    db::change_bans(&pool, a.id, &good).await.expect("change");
    let changed = db::active_ban(&pool, two.id)
        .await
        .expect("get")
        .expect("ban");
    assert_eq!(changed.reason, "changed");
    assert!(changed.ends_at.is_none());

    let lift = LiftBans {
        user_ids: vec![one.id, clean.id],
    };
    assert!(db::lift_bans(&pool, a.id, &lift).await.is_err());
    assert!(db::active_ban(&pool, one.id).await.expect("kept").is_some());

    let actions = db::list_moderation_actions(&pool, two.id)
        .await
        .expect("audit");
    let names: Vec<&str> = actions.iter().map(|m| m.action.as_str()).collect();
    assert_eq!(names, ["ban_changed", "ban"]);
}

#[tokio::test]
async fn trash_retention_waits_for_an_active_ban() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "actor").await;
    let target = user(&pool, "target").await;
    db::ensure_user_settings(&pool, target.id)
        .await
        .expect("settings");
    let id = db::create_bookmark(
        &pool,
        target.id,
        CreateBookmark {
            url: "https://old.example/".into(),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id;
    sqlx::query("UPDATE bookmarks SET deleted_at = now() - interval '10 years' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("age");
    let exists = || async {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM bookmarks WHERE id = $1)")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("exists")
    };

    db::ban_user(&pool, a.id, target.id, &ban(None))
        .await
        .expect("ban");
    db::purge_expired_trash(&pool).await.expect("purge");
    assert!(exists().await, "kept during the ban");

    let lift = LiftBans {
        user_ids: vec![target.id],
    };
    db::lift_bans(&pool, a.id, &lift).await.expect("lift");
    db::purge_expired_trash(&pool).await.expect("purge");
    assert!(!exists().await, "purged once the ban ends");
}

#[tokio::test]
async fn account_list_reports_ban_state() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "actor").await;
    let target = user(&pool, "target").await;
    db::ban_user(&pool, a.id, target.id, &ban(Some(3)))
        .await
        .expect("ban");
    let users = db::list_admin_users(&pool).await.expect("list");
    let row = users.iter().find(|u| u.id == target.id).expect("listed");
    assert!(row.banned);
    assert!(row.banned_until.is_some());
    assert!(db::list_bans(&pool)
        .await
        .expect("bans")
        .iter()
        .any(|b| b.user_id == target.id));
}
