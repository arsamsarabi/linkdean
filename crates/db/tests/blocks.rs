//! User block (hard cut) and mute (one-way quiet).
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CategoryView, CreateBookmark, CreateCategory, CreateCategoryShare, CreateCategoryShareOutcome,
    CreateDashboard, CreateDashboardShare, CreateSocialComment, Error, NotificationKind, Result,
    ShareAccess, SocialSubjectType, UpsertSocialRating, User,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping blocks tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn email(prefix: &str) -> String {
    format!("{prefix}-{}@blocks.test", Uuid::new_v4())
}

async fn user(pool: &PgPool, prefix: &str, display_name: Option<&str>) -> User {
    db::create_user(
        pool,
        &email(prefix),
        Some("password-long"),
        display_name,
        None,
        None,
    )
    .await
    .expect("user")
}

async fn category(pool: &PgPool, owner: &User) -> CategoryView {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: format!("Blocks-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
            icon: None,
        },
    )
    .await
    .expect("category")
}

async fn share(pool: &PgPool, owner: &User, category_id: Uuid, email: &str) -> Result<()> {
    db::create_share(
        pool,
        owner.id,
        category_id,
        CreateCategoryShare {
            email: email.to_string(),
            access: ShareAccess::Write,
        },
    )
    .await
    .map(|_| ())
}

async fn shared_bookmark(pool: &PgPool, owner: &User, category_id: Uuid) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: format!("https://blocks.example/{}/", Uuid::new_v4()),
            title: Some("Shared".into()),
            description: None,
            notes: None,
            category_id: Some(category_id),
            category_ids: vec![category_id],
            tags: vec![],
            ..Default::default()
        },
        None,
    )
    .await
    .expect("bookmark")
    .id
}

async fn comment(pool: &PgPool, author: &User, bookmark_id: Uuid, body: &str) {
    db::add_comment(
        pool,
        author.id,
        SocialSubjectType::Bookmark,
        bookmark_id,
        CreateSocialComment { body: body.into() },
    )
    .await
    .expect("comment");
}

async fn shared_category_ids(pool: &PgPool, invitee: &User) -> Vec<Uuid> {
    db::list_shared_categories(pool, invitee.id, true)
        .await
        .expect("shared categories")
        .into_iter()
        .map(|(c, _)| c.id)
        .collect()
}

#[tokio::test]
async fn block_cuts_follows_mutes_and_shares_both_ways() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a", Some("Ana")).await;
    let b = user(&pool, "b", Some("Bo")).await;

    db::follow_user(&pool, a.id, b.id)
        .await
        .expect("a follows b");
    db::follow_user(&pool, b.id, a.id)
        .await
        .expect("b follows a");
    db::mute_user(&pool, b.id, a.id).await.expect("b mutes a");

    let a_cat = category(&pool, &a).await;
    let b_cat = category(&pool, &b).await;
    share(&pool, &a, a_cat.id, &b.email)
        .await
        .expect("a shares");
    share(&pool, &b, b_cat.id, &a.email)
        .await
        .expect("b shares");

    let a_dash = db::create_dashboard(
        &pool,
        a.id,
        CreateDashboard {
            title: Some(format!("Board-{}", Uuid::new_v4())),
        },
        None,
    )
    .await
    .expect("dashboard");
    db::create_dashboard_share(
        &pool,
        a.id,
        a_dash.id,
        CreateDashboardShare {
            email: b.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("dashboard share");

    let relation = db::block_user(&pool, a.id, b.id).await.expect("block");
    assert!(relation.blocked);

    assert!(!db::is_following(&pool, a.id, b.id).await.expect("follow a"));
    assert!(!db::is_following(&pool, b.id, a.id).await.expect("follow b"));
    assert!(!db::is_muted(&pool, b.id, a.id).await.expect("mute cleared"));
    assert!(!shared_category_ids(&pool, &a).await.contains(&b_cat.id));
    assert!(!shared_category_ids(&pool, &b).await.contains(&a_cat.id));
    let b_dashboards = db::list_shared_dashboards(&pool, b.id)
        .await
        .expect("dashboards");
    assert!(!b_dashboards.iter().any(|(d, _)| d.id == a_dash.id));

    // Revoke is silent: the blocked user learns nothing from the bell.
    let b_notes = db::list_notifications(&pool, b.id, 50)
        .await
        .expect("notes");
    assert!(!b_notes
        .iter()
        .any(|n| n.kind == NotificationKind::ShareRevoked));
}

#[tokio::test]
async fn block_rejects_new_links_either_way_until_unblocked() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a", Some("Ana")).await;
    let b = user(&pool, "b", Some("Bo")).await;
    let a_cat = category(&pool, &a).await;
    let b_cat = category(&pool, &b).await;

    db::block_user(&pool, a.id, b.id).await.expect("block");

    assert!(matches!(
        db::follow_user(&pool, b.id, a.id).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::follow_user(&pool, a.id, b.id).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        share(&pool, &b, b_cat.id, &a.email).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        share(&pool, &a, a_cat.id, &b.email).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::mute_user(&pool, b.id, a.id).await,
        Err(Error::BadRequest(_))
    ));

    // Only the blocker can lift it.
    let by_blocked = db::unblock_user(&pool, b.id, a.id)
        .await
        .expect("noop unblock");
    assert!(!by_blocked.blocked);
    assert!(db::is_blocked(&pool, a.id, b.id)
        .await
        .expect("still blocked"));

    // block-status never tells the blocked user who blocked them.
    let seen_by_blocked = db::block_relation(&pool, b.id, a.id)
        .await
        .expect("relation");
    assert!(!seen_by_blocked.blocked);

    let unblocked = db::unblock_user(&pool, a.id, b.id).await.expect("unblock");
    assert!(!unblocked.blocked);
    db::follow_user(&pool, b.id, a.id)
        .await
        .expect("follow after unblock");
    share(&pool, &a, a_cat.id, &b.email)
        .await
        .expect("share after unblock");
}

#[tokio::test]
async fn self_block_and_self_mute_are_rejected() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a", None).await;
    assert!(matches!(
        db::block_user(&pool, a.id, a.id).await,
        Err(Error::BadRequest(_))
    ));
    assert!(matches!(
        db::mute_user(&pool, a.id, a.id).await,
        Err(Error::BadRequest(_))
    ));
}

#[tokio::test]
async fn unknown_target_is_not_found() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a", None).await;
    let ghost = Uuid::new_v4();
    assert!(matches!(
        db::block_user(&pool, a.id, ghost).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        db::mute_user(&pool, a.id, ghost).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn blocking_is_idempotent() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let a = user(&pool, "a", Some("Ana")).await;
    let b = user(&pool, "b", Some("Bo")).await;
    db::block_user(&pool, a.id, b.id).await.expect("first");
    db::block_user(&pool, a.id, b.id).await.expect("second");
    assert_eq!(db::list_blocked(&pool, a.id).await.expect("list").len(), 1);
}

#[tokio::test]
async fn block_revokes_pending_invites_and_claim_skips_them() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own", Some("Own")).await;
    let cat = category(&pool, &owner).await;

    // Mixed case: invites store a normalized address.
    let invited = email("Later");
    let outcome = db::create_share(
        &pool,
        owner.id,
        cat.id,
        CreateCategoryShare {
            email: invited.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("pending invite");
    assert!(matches!(
        outcome,
        CreateCategoryShareOutcome::Pending { .. }
    ));

    let later = db::create_user(&pool, &invited, Some("password-long"), None, None, None)
        .await
        .expect("register");

    db::block_user(&pool, later.id, owner.id)
        .await
        .expect("block");

    let (revoked,): (bool,) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL FROM pending_share_invites WHERE invited_by_user_id = $1",
    )
    .bind(owner.id)
    .fetch_one(&pool)
    .await
    .expect("invite row");
    assert!(revoked, "block must revoke the open invite");

    // Reopen it to reach the claim-time guard, e.g. invite raced the block.
    sqlx::query("UPDATE pending_share_invites SET revoked_at = NULL WHERE invited_by_user_id = $1")
        .bind(owner.id)
        .execute(&pool)
        .await
        .expect("reopen");

    let claimed = db::claim_pending_share_invites(&pool, &invited, later.id)
        .await
        .expect("claim");
    assert_eq!(claimed, 0);
    assert!(!shared_category_ids(&pool, &later).await.contains(&cat.id));
}

#[tokio::test]
async fn mute_hides_for_the_muter_only_and_unmute_restores() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own", Some("Own")).await;
    let loud = user(&pool, "loud", Some("Loud")).await;
    let quiet = user(&pool, "quiet", Some("Quiet")).await;
    let cat = category(&pool, &owner).await;
    share(&pool, &owner, cat.id, &loud.email)
        .await
        .expect("share loud");
    share(&pool, &owner, cat.id, &quiet.email)
        .await
        .expect("share quiet");
    let bm = shared_bookmark(&pool, &owner, cat.id).await;

    comment(&pool, &loud, bm, "hello").await;
    db::set_like(&pool, loud.id, SocialSubjectType::Bookmark, bm, true)
        .await
        .expect("like");
    db::upsert_rating(
        &pool,
        loud.id,
        SocialSubjectType::Bookmark,
        bm,
        UpsertSocialRating { rating: 5 },
    )
    .await
    .expect("rating");

    db::mute_user(&pool, quiet.id, loud.id).await.expect("mute");

    let muted = db::get_social_summary(&pool, quiet.id, SocialSubjectType::Bookmark, bm)
        .await
        .expect("muter summary");
    assert_eq!(muted.comments_count, 0);
    assert_eq!(muted.likes_count, 0);
    assert_eq!(muted.rating_count, 0);

    // One-way: the muted user and everyone else still see it.
    for viewer in [&loud, &owner] {
        let summary = db::get_social_summary(&pool, viewer.id, SocialSubjectType::Bookmark, bm)
            .await
            .expect("other summary");
        assert_eq!(summary.comments_count, 1);
        assert_eq!(summary.likes_count, 1);
        assert_eq!(summary.rating_count, 1);
    }
    // Muting does not stop the muted user reading or engaging.
    db::set_like(&pool, loud.id, SocialSubjectType::Bookmark, bm, false)
        .await
        .expect("still can act");

    let unmuted = db::unmute_user(&pool, quiet.id, loud.id)
        .await
        .expect("unmute");
    assert!(!unmuted.muted);
    let restored = db::get_social_summary(&pool, quiet.id, SocialSubjectType::Bookmark, bm)
        .await
        .expect("restored");
    assert_eq!(restored.comments_count, 1);
}

#[tokio::test]
async fn third_party_folder_hides_comments_between_blocked_users() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own", Some("Own")).await;
    let a = user(&pool, "a", Some("Ana")).await;
    let b = user(&pool, "b", Some("Bo")).await;
    let cat = category(&pool, &owner).await;
    share(&pool, &owner, cat.id, &a.email)
        .await
        .expect("share a");
    share(&pool, &owner, cat.id, &b.email)
        .await
        .expect("share b");
    let bm = shared_bookmark(&pool, &owner, cat.id).await;
    comment(&pool, &a, bm, "from a").await;
    comment(&pool, &b, bm, "from b").await;

    db::block_user(&pool, a.id, b.id).await.expect("block");

    // Both keep the owner's share; each loses the other's comment.
    for (viewer, visible) in [(&a, "from a"), (&b, "from b")] {
        let summary = db::get_social_summary(&pool, viewer.id, SocialSubjectType::Bookmark, bm)
            .await
            .expect("summary");
        let bodies: Vec<&str> = summary.comments.iter().map(|c| c.body.as_str()).collect();
        assert_eq!(bodies, vec![visible]);
    }
    let owner_view = db::get_social_summary(&pool, owner.id, SocialSubjectType::Bookmark, bm)
        .await
        .expect("owner summary");
    assert_eq!(owner_view.comments_count, 2);
}

#[tokio::test]
async fn blocked_user_cannot_engage_with_owner_subject() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own", Some("Own")).await;
    let member = user(&pool, "mem", Some("Mem")).await;
    let cat = category(&pool, &owner).await;
    share(&pool, &owner, cat.id, &member.email)
        .await
        .expect("share");
    let bm = shared_bookmark(&pool, &owner, cat.id).await;

    db::block_user(&pool, owner.id, member.id)
        .await
        .expect("block");

    let like = db::set_like(&pool, member.id, SocialSubjectType::Bookmark, bm, true).await;
    assert!(like.is_err(), "blocked user must not reach the subject");
    let read = db::get_social_summary(&pool, member.id, SocialSubjectType::Bookmark, bm).await;
    assert!(read.is_err(), "blocked user must not read the subject");
}

#[tokio::test]
async fn blocked_user_keeps_folder_access_but_cannot_engage_with_blockers_bookmark() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let host = user(&pool, "host", Some("Host")).await;
    let author = user(&pool, "auth", Some("Auth")).await;
    let member = user(&pool, "mem", Some("Mem")).await;
    let cat = category(&pool, &host).await;
    share(&pool, &host, cat.id, &author.email)
        .await
        .expect("share author");
    share(&pool, &host, cat.id, &member.email)
        .await
        .expect("share member");
    let bm = shared_bookmark(&pool, &author, cat.id).await;

    db::block_user(&pool, author.id, member.id)
        .await
        .expect("block");

    // The host's share is untouched, so only the owner check can stop this.
    assert!(shared_category_ids(&pool, &member).await.contains(&cat.id));
    assert!(matches!(
        db::set_like(&pool, member.id, SocialSubjectType::Bookmark, bm, true).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::get_social_summary(&pool, member.id, SocialSubjectType::Bookmark, bm).await,
        Err(Error::Forbidden)
    ));
    db::set_like(&pool, host.id, SocialSubjectType::Bookmark, bm, true)
        .await
        .expect("unrelated member unaffected");
}

#[tokio::test]
async fn notifications_hidden_by_mute_and_block_with_unread_count_in_step() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "own", Some("Own")).await;
    let fan = user(&pool, "fan", Some("Fan")).await;

    db::follow_user(&pool, fan.id, owner.id)
        .await
        .expect("follow");
    assert_eq!(
        db::unread_notification_count(&pool, owner.id)
            .await
            .expect("count"),
        1
    );

    db::mute_user(&pool, owner.id, fan.id).await.expect("mute");
    assert_eq!(
        db::unread_notification_count(&pool, owner.id)
            .await
            .expect("muted count"),
        0
    );
    assert!(db::list_notifications(&pool, owner.id, 20)
        .await
        .expect("muted list")
        .is_empty());

    // New events from a muted actor are never written, so unmute shows only the old one.
    db::unfollow_user(&pool, fan.id, owner.id)
        .await
        .expect("unfollow");
    db::follow_user(&pool, fan.id, owner.id)
        .await
        .expect("refollow");
    db::unmute_user(&pool, owner.id, fan.id)
        .await
        .expect("unmute");
    assert_eq!(
        db::unread_notification_count(&pool, owner.id)
            .await
            .expect("unmuted count"),
        1
    );

    db::block_user(&pool, fan.id, owner.id)
        .await
        .expect("block by fan");
    assert_eq!(
        db::unread_notification_count(&pool, owner.id)
            .await
            .expect("blocked count"),
        0,
        "block hides either way"
    );
}

#[tokio::test]
async fn lists_show_only_my_targets_newest_first_with_labels() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = user(&pool, "me", Some("Me")).await;
    let named = user(&pool, "named", Some("Named")).await;
    let unnamed = user(&pool, "unnamed", None).await;
    let other = user(&pool, "other", Some("Other")).await;

    db::block_user(&pool, me.id, named.id)
        .await
        .expect("block named");
    db::block_user(&pool, me.id, unnamed.id)
        .await
        .expect("block unnamed");
    db::block_user(&pool, other.id, me.id)
        .await
        .expect("other blocks me");
    db::mute_user(&pool, other.id, named.id)
        .await
        .expect("other mutes");

    let blocked = db::list_blocked(&pool, me.id).await.expect("blocked");
    let ids: Vec<Uuid> = blocked.iter().map(|u| u.id).collect();
    assert_eq!(ids, vec![unnamed.id, named.id]);
    assert_eq!(blocked[1].display_name, "Named");
    let local_part = unnamed.email.split('@').next().expect("local part");
    assert_eq!(blocked[0].display_name, local_part);

    assert!(db::list_muted(&pool, me.id)
        .await
        .expect("muted")
        .is_empty());
}
