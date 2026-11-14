//! Per-kind notification email preferences and the pending-email queue.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateCategory, CreateCategoryShare, EmailDelivery, NotificationKind, NotificationSubjectType,
    ShareAccess, User,
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
    for pref in prefs {
        assert_eq!(
            pref.enabled,
            pref.kind.email_by_default(),
            "{:?}",
            pref.kind
        );
        assert_eq!(pref.delivery, EmailDelivery::Instant);
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
    db::set_notification_email_pref(
        &pool,
        u.id,
        NotificationKind::SocialComment,
        true,
        EmailDelivery::Instant,
    )
    .await
    .expect("on");
    db::set_notification_email_pref(
        &pool,
        u.id,
        NotificationKind::ShareInvite,
        false,
        EmailDelivery::Instant,
    )
    .await
    .expect("off");
    // Setting it again replaces the row.
    db::set_notification_email_pref(
        &pool,
        u.id,
        NotificationKind::ShareInvite,
        false,
        EmailDelivery::Instant,
    )
    .await
    .expect("off again");
    let prefs = db::get_notification_email_prefs(&pool, u.id)
        .await
        .expect("prefs");
    let get = |k| prefs.iter().find(|p| p.kind == k).unwrap().enabled;
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
    db::set_notification_email_pref(
        &pool,
        opted_out.id,
        NotificationKind::ShareInvite,
        false,
        EmailDelivery::Instant,
    )
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
    db::set_notification_email_pref(
        &pool,
        wants.id,
        NotificationKind::SocialComment,
        true,
        EmailDelivery::Instant,
    )
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
    db::set_notification_email_pref(
        &pool,
        to.id,
        NotificationKind::SocialLike,
        true,
        EmailDelivery::Instant,
    )
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
        db::set_notification_email_pref(
            &pool,
            u.id,
            NotificationKind::SocialComment,
            true,
            EmailDelivery::Instant,
        )
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
    db::set_notification_email_pref(
        &pool,
        later.id,
        NotificationKind::SocialComment,
        true,
        EmailDelivery::Instant,
    )
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

#[tokio::test]
async fn a_new_share_invite_email_can_be_found_and_cancelled() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let owner = user(&pool, "finder", true).await;
    let invitee = user(&pool, "found", true).await;
    let cat = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: format!("Find-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("cat");
    let share = || {
        db::create_share(
            &pool,
            owner.id,
            cat.id,
            CreateCategoryShare {
                email: invitee.email.clone(),
                access: ShareAccess::Read,
            },
        )
    };

    let since = db::database_now(&pool).await.expect("now");
    share().await.expect("share");
    let found = db::find_new_share_invite_email(
        &pool,
        owner.id,
        NotificationSubjectType::Category,
        cat.id,
        since,
    )
    .await
    .expect("find")
    .expect("an invite email");

    // Sharing again sends no second invite, so nothing new is found.
    let again = db::database_now(&pool).await.expect("now");
    share().await.expect("share again");
    assert!(db::find_new_share_invite_email(
        &pool,
        owner.id,
        NotificationSubjectType::Category,
        cat.id,
        again,
    )
    .await
    .expect("find")
    .is_none());

    assert!(db::cancel_notification_email(&pool, found)
        .await
        .expect("cancel"));
    assert!(!db::cancel_notification_email(&pool, found)
        .await
        .expect("cancel twice"));
    let s = states(&pool, invitee.id, NotificationKind::ShareInvite).await;
    assert_eq!(s, [(found, "none".to_string())]);
}

async fn set_timezone(pool: &PgPool, user_id: Uuid, tz: Option<&str>) {
    sqlx::query("UPDATE users SET timezone = $2 WHERE id = $1")
        .bind(user_id)
        .bind(tz)
        .execute(pool)
        .await
        .expect("timezone");
}

/// The hour it is now in `tz`, so a test can ask for a digest hour already passed or still ahead.
async fn local_hour(pool: &PgPool, tz: &str) -> u32 {
    let (h,): (f64,) = sqlx::query_as("SELECT extract(hour FROM now() AT TIME ZONE $1)::float8")
        .bind(tz)
        .fetch_one(pool)
        .await
        .expect("hour");
    h as u32
}

/// Every due digest, for any recipient, keeping only those to `users`.
async fn digests_for(pool: &PgPool, hour: u32, users: &[Uuid]) -> Vec<db::NotificationDigest> {
    db::claim_due_notification_digests(pool, hour, 500)
        .await
        .expect("claim digests")
        .into_iter()
        .filter(|d| users.contains(&d.recipient_user_id))
        .collect()
}

#[tokio::test]
async fn a_daily_kind_waits_for_the_digest_and_goes_out_once_a_day() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let to = user(&pool, "digest-to", true).await;
    let from = user(&pool, "digest-from", true).await;
    set_timezone(&pool, to.id, Some("Asia/Tehran")).await;
    db::set_notification_email_pref(
        &pool,
        to.id,
        NotificationKind::SocialLike,
        true,
        EmailDelivery::Daily,
    )
    .await
    .expect("daily");
    notify(&pool, to.id, from.id, NotificationKind::SocialLike).await;
    notify(&pool, to.id, from.id, NotificationKind::SocialLike).await;
    let st = states(&pool, to.id, NotificationKind::SocialLike).await;
    assert!(st.iter().all(|(_, s)| s == "digest"), "{st:?}");
    // Not sent one by one.
    assert!(claim_all(&pool)
        .await
        .iter()
        .all(|n| n.recipient_user_id != to.id));

    let now = local_hour(&pool, "Asia/Tehran").await;
    if now < 23 {
        // The digest hour is still ahead in their zone: nothing yet.
        assert!(digests_for(&pool, now + 1, &[to.id]).await.is_empty());
    }
    let sent = digests_for(&pool, now, &[to.id]).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].items.len(), 2);
    assert_eq!(sent[0].recipient_email, to.email);
    assert!(sent[0].items[0].created_at <= sent[0].items[1].created_at);
    let st = states(&pool, to.id, NotificationKind::SocialLike).await;
    assert!(st.iter().all(|(_, s)| s == "queued"), "{st:?}");

    // A later notification the same day waits for tomorrow.
    notify(&pool, to.id, from.id, NotificationKind::SocialLike).await;
    assert!(digests_for(&pool, now, &[to.id]).await.is_empty());
    // Once the last digest is a day old, it goes out.
    sqlx::query(
        "UPDATE notification_digest_runs SET last_sent_at = now() - interval '1 day' WHERE user_id = $1",
    )
    .bind(to.id)
    .execute(&pool)
    .await
    .expect("age");
    let sent = digests_for(&pool, now, &[to.id]).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].items.len(), 1);
}

#[tokio::test]
async fn a_digest_uses_utc_without_a_valid_time_zone_and_skips_muted_actors() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let bad_zone = user(&pool, "digest-badzone", true).await;
    let muter = user(&pool, "digest-muter", true).await;
    let from = user(&pool, "digest-actor", true).await;
    set_timezone(&pool, bad_zone.id, Some("Not/AZone")).await;
    for u in [&bad_zone, &muter] {
        db::set_notification_email_pref(
            &pool,
            u.id,
            NotificationKind::SocialComment,
            true,
            EmailDelivery::Daily,
        )
        .await
        .expect("daily");
        notify(&pool, u.id, from.id, NotificationKind::SocialComment).await;
    }
    db::mute_user(&pool, muter.id, from.id).await.expect("mute");
    let now = local_hour(&pool, "UTC").await;
    let sent = digests_for(&pool, now, &[bad_zone.id, muter.id]).await;
    assert_eq!(sent.len(), 1, "the muter's only item is dropped");
    assert_eq!(sent[0].recipient_user_id, bad_zone.id);
    let st = states(&pool, muter.id, NotificationKind::SocialComment).await;
    assert!(st.iter().all(|(_, s)| s == "queued"), "{st:?}");
}

#[tokio::test]
async fn changing_delivery_moves_waiting_emails() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let to = user(&pool, "move-to", true).await;
    let from = user(&pool, "move-from", true).await;
    let kind = NotificationKind::SharedEdit;
    db::set_notification_email_pref(&pool, to.id, kind, true, EmailDelivery::Daily)
        .await
        .expect("daily");
    notify(&pool, to.id, from.id, kind).await;
    assert_eq!(states(&pool, to.id, kind).await[0].1, "digest");
    db::set_notification_email_pref(&pool, to.id, kind, true, EmailDelivery::Instant)
        .await
        .expect("instant");
    assert_eq!(states(&pool, to.id, kind).await[0].1, "pending");
    db::set_notification_email_pref(&pool, to.id, kind, true, EmailDelivery::Daily)
        .await
        .expect("daily again");
    assert_eq!(states(&pool, to.id, kind).await[0].1, "digest");
    db::set_notification_email_pref(&pool, to.id, kind, false, EmailDelivery::Daily)
        .await
        .expect("off");
    assert_eq!(states(&pool, to.id, kind).await[0].1, "none");
    let prefs = db::get_notification_email_prefs(&pool, to.id)
        .await
        .expect("prefs");
    let p = prefs.iter().find(|p| p.kind == kind).unwrap();
    assert!(!p.enabled);
    assert_eq!(p.delivery, EmailDelivery::Daily);

    // Invites and moderation changes never wait.
    for k in [
        NotificationKind::ShareInvite,
        NotificationKind::ModerationChanges,
    ] {
        assert!(
            db::set_notification_email_pref(&pool, to.id, k, true, EmailDelivery::Daily)
                .await
                .is_err(),
            "{k:?}"
        );
    }
}

#[tokio::test]
async fn unsubscribing_turns_off_a_kind_or_the_digest() {
    use linkdean_core::unsubscribe::UnsubscribeScope;
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let _turn = SERIAL.lock().await;
    let to = user(&pool, "unsub-to", true).await;
    let from = user(&pool, "unsub-from", true).await;
    // A default-on kind with no saved row.
    notify(&pool, to.id, from.id, NotificationKind::ModerationChanges).await;
    let out = db::unsubscribe(
        &pool,
        to.id,
        UnsubscribeScope::Kind(NotificationKind::ModerationChanges),
    )
    .await
    .expect("unsubscribe kind");
    assert_eq!(out.kinds, vec![NotificationKind::ModerationChanges]);
    assert_eq!(
        states(&pool, to.id, NotificationKind::ModerationChanges).await[0].1,
        "none"
    );

    for k in [NotificationKind::SocialLike, NotificationKind::SocialRating] {
        db::set_notification_email_pref(&pool, to.id, k, true, EmailDelivery::Daily)
            .await
            .expect("daily");
        notify(&pool, to.id, from.id, k).await;
    }
    db::set_notification_email_pref(
        &pool,
        to.id,
        NotificationKind::SocialComment,
        true,
        EmailDelivery::Instant,
    )
    .await
    .expect("instant");
    let mut out = db::unsubscribe(&pool, to.id, UnsubscribeScope::Digest)
        .await
        .expect("unsubscribe digest");
    out.kinds.sort_by_key(|k| k.as_str());
    assert_eq!(
        out.kinds,
        vec![NotificationKind::SocialLike, NotificationKind::SocialRating]
    );
    let prefs = db::get_notification_email_prefs(&pool, to.id)
        .await
        .expect("prefs");
    let on = |k| prefs.iter().find(|p| p.kind == k).unwrap().enabled;
    assert!(!on(NotificationKind::SocialLike));
    assert!(!on(NotificationKind::SocialRating));
    assert!(
        on(NotificationKind::SocialComment),
        "an instant kind is left on"
    );
    assert_eq!(
        states(&pool, to.id, NotificationKind::SocialLike).await[0].1,
        "none"
    );
    // Unsubscribing twice is fine; a missing user is not found.
    db::unsubscribe(&pool, to.id, UnsubscribeScope::Digest)
        .await
        .expect("again");
    assert!(matches!(
        db::unsubscribe(&pool, Uuid::new_v4(), UnsubscribeScope::Digest).await,
        Err(linkdean_core::Error::NotFound)
    ));
}
