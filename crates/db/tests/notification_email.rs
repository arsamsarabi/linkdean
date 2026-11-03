//! Per-kind notification email preferences and the pending-email queue.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateCategory, CreateCategoryShare, NotificationKind, NotificationSubjectType, ShareAccess,
    User,
};
use linkdean_db::{self as db, NotificationSubject};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Claiming takes every pending row in the database, so the tests in this file take turns.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping notification email tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str, verified: bool) -> User {
    let email = format!("{prefix}-{}@notif-email.test", Uuid::new_v4());
    let u = db::create_user(
        pool,
        &email,
        Some("password-long"),
        Some(prefix),
        None,
        None,
    )
    .await
    .expect("user");
    if verified {
        db::mark_email_verified(pool, u.id).await.expect("verify");
    }
    u
}

async fn notify(pool: &PgPool, to: Uuid, from: Uuid, kind: NotificationKind) {
    db::try_notify(
        pool,
        to,
        Some(from),
        kind,
        NotificationSubject {
            subject_type: NotificationSubjectType::Bookmark,
            subject_id: Uuid::new_v4(),
            subject_title: "A bookmark",
        },
        "Someone did something",
        json!({}),
    )
    .await;
}

/// (id, email_state) of every notification to `user_id` of `kind`.
async fn states(pool: &PgPool, user_id: Uuid, kind: NotificationKind) -> Vec<(Uuid, String)> {
    sqlx::query_as(
        "SELECT id, email_state FROM notifications WHERE recipient_user_id = $1 AND kind = $2",
    )
    .bind(user_id)
    .bind(kind.as_str())
    .fetch_all(pool)
    .await
    .expect("states")
}

async fn claim_all(pool: &PgPool) -> Vec<db::PendingNotificationEmail> {
    let mut all = Vec::new();
    loop {
        let batch = db::claim_pending_notification_emails(pool, 200)
            .await
            .expect("claim");
        if batch.is_empty() {
            return all;
        }
        all.extend(batch);
    }
}

#[tokio::test]
async fn every_kind_has_its_default() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = user(&pool, "defaults", true).await;
    let prefs = db::get_notification_email_prefs(&pool, u.id)
        .await
        .expect("prefs");
    assert_eq!(prefs.len(), NotificationKind::ALL.len());
    for (kind, enabled) in prefs {
        assert_eq!(enabled, kind.email_by_default(), "{kind:?}");
    }
    assert!(NotificationKind::ShareInvite.email_by_default());
    assert!(NotificationKind::ModerationChanges.email_by_default());
    assert!(!NotificationKind::SocialComment.email_by_default());
}

#[tokio::test]
async fn a_preference_overrides_the_default() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let u = user(&pool, "override", true).await;
    db::set_notification_email_pref(&pool, u.id, NotificationKind::SocialComment, true)
        .await
        .expect("on");
    db::set_notification_email_pref(&pool, u.id, NotificationKind::ShareInvite, false)
        .await
        .expect("off");
    // Setting it again replaces the row.
    db::set_notification_email_pref(&pool, u.id, NotificationKind::ShareInvite, false)
        .await
        .expect("off again");
    let prefs = db::get_notification_email_prefs(&pool, u.id)
        .await
        .expect("prefs");
    let get = |k| prefs.iter().find(|(kind, _)| *kind == k).unwrap().1;
    assert!(get(NotificationKind::SocialComment));
    assert!(!get(NotificationKind::ShareInvite));
    assert!(get(NotificationKind::ModerationChanges));
}

#[tokio::test]
async fn a_share_invite_is_pending_only_when_wanted_and_verified() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let owner = user(&pool, "owner", true).await;
    let wants = user(&pool, "wants", true).await;
    let opted_out = user(&pool, "optout", true).await;
    db::set_notification_email_pref(&pool, opted_out.id, NotificationKind::ShareInvite, false)
        .await
        .expect("off");

    let cat = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: format!("Shared-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("cat");
    for invitee in [&wants, &opted_out] {
        db::create_share(
            &pool,
            owner.id,
            cat.id,
            CreateCategoryShare {
                email: invitee.email.clone(),
                access: ShareAccess::Read,
            },
        )
        .await
        .expect("share");
    }
    let s = states(&pool, wants.id, NotificationKind::ShareInvite).await;
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].1, "pending");
    let s = states(&pool, opted_out.id, NotificationKind::ShareInvite).await;
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].1, "none");

    // An unverified address never gets mail, even for a kind that is on by default.
    let unverified = user(&pool, "unverified", false).await;
    notify(
        &pool,
        unverified.id,
        owner.id,
        NotificationKind::ShareInvite,
    )
    .await;
    let s = states(&pool, unverified.id, NotificationKind::ShareInvite).await;
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].1, "none");

    // A kind that is off by default stays off until the user turns it on.
    notify(&pool, wants.id, owner.id, NotificationKind::SocialComment).await;
    db::set_notification_email_pref(&pool, wants.id, NotificationKind::SocialComment, true)
        .await
        .expect("on");
    notify(&pool, wants.id, owner.id, NotificationKind::SocialComment).await;
    let mut s: Vec<String> = states(&pool, wants.id, NotificationKind::SocialComment)
        .await
        .into_iter()
        .map(|(_, st)| st)
        .collect();
    s.sort();
    assert_eq!(s, ["none", "pending"]);
}

#[tokio::test]
async fn claiming_queues_each_pending_email_once() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let actor = user(&pool, "actor", true).await;
    let to = user(&pool, "claimer", true).await;
    db::set_notification_email_pref(&pool, to.id, NotificationKind::SocialLike, true)
        .await
        .expect("on");
    notify(&pool, to.id, actor.id, NotificationKind::SocialLike).await;
    notify(&pool, to.id, actor.id, NotificationKind::SocialLike).await;
    notify(&pool, to.id, actor.id, NotificationKind::SocialRating).await;

    let claimed: Vec<_> = claim_all(&pool)
        .await
        .into_iter()
        .filter(|p| p.recipient_user_id == to.id)
        .collect();
    assert_eq!(claimed.len(), 2, "only the kind that is on");
    let first = &claimed[0];
    assert_eq!(first.kind, NotificationKind::SocialLike);
    assert_eq!(first.recipient_email, to.email);
    assert_eq!(first.body, "Someone did something");
    assert_eq!(first.subject_type, NotificationSubjectType::Bookmark);
    assert_eq!(first.subject_title, "A bookmark");
    assert_eq!(first.actor_user_id, Some(actor.id));
    assert_eq!(first.actor_label.as_deref(), Some("actor"));
    assert_ne!(claimed[0].id, claimed[1].id);

    for (_, st) in states(&pool, to.id, NotificationKind::SocialLike).await {
        assert_eq!(st, "queued");
    }
    assert!(claim_all(&pool)
        .await
        .iter()
        .all(|p| p.recipient_user_id != to.id));
}

#[tokio::test]
async fn muted_and_blocked_actors_send_no_email() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let actor = user(&pool, "noisy", true).await;
    let muter = user(&pool, "muter", true).await;
    let blocker = user(&pool, "blocker", true).await;
    for u in [&muter, &blocker] {
        db::set_notification_email_pref(&pool, u.id, NotificationKind::SocialComment, true)
            .await
            .expect("on");
    }
    db::mute_user(&pool, muter.id, actor.id)
        .await
        .expect("mute");
    db::block_user(&pool, blocker.id, actor.id)
        .await
        .expect("block");
    notify(&pool, muter.id, actor.id, NotificationKind::SocialComment).await;
    notify(&pool, blocker.id, actor.id, NotificationKind::SocialComment).await;
    assert!(states(&pool, muter.id, NotificationKind::SocialComment)
        .await
        .is_empty());
    assert!(states(&pool, blocker.id, NotificationKind::SocialComment)
        .await
        .is_empty());

    // Muting after the notification was made still stops its email.
    let later = user(&pool, "later", true).await;
    db::set_notification_email_pref(&pool, later.id, NotificationKind::SocialComment, true)
        .await
        .expect("on");
    notify(&pool, later.id, actor.id, NotificationKind::SocialComment).await;
    db::mute_user(&pool, later.id, actor.id)
        .await
        .expect("mute");
    assert!(claim_all(&pool)
        .await
        .iter()
        .all(|p| p.recipient_user_id != later.id));
}

#[tokio::test]
async fn an_unverified_address_or_discarded_backlog_is_not_emailed() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let actor = user(&pool, "sender", true).await;
    let to = user(&pool, "changer", true).await;
    notify(&pool, to.id, actor.id, NotificationKind::ShareInvite).await;
    // The address changed (and so is unverified again) before the mailer ran.
    sqlx::query("UPDATE users SET email_verified_at = NULL WHERE id = $1")
        .bind(to.id)
        .execute(&pool)
        .await
        .expect("unverify");
    assert!(claim_all(&pool)
        .await
        .iter()
        .all(|p| p.recipient_user_id != to.id));

    // With mail off, the mailer discards pending rows instead of letting them pile up.
    let other = user(&pool, "backlog", true).await;
    notify(&pool, other.id, actor.id, NotificationKind::ShareInvite).await;
    while db::discard_pending_notification_emails(&pool, 200)
        .await
        .expect("discard")
        > 0
    {}
    let s = states(&pool, other.id, NotificationKind::ShareInvite).await;
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].1, "none");
    assert!(claim_all(&pool)
        .await
        .iter()
        .all(|p| p.recipient_user_id != other.id));
}
