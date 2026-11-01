//! Account bans and moderation of what a banned user exposes.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use chrono::{Duration, Utc};
use linkdean_core::{
    ChangeBans, CreateBan, CreateBookmark, CreateCategory, Error, LiftBans, ModerateAction,
    ModerateRequest, UpdateAdminUser, User, VaultKeyring,
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

struct Exposed {
    pool: PgPool,
    admin: User,
    owner: User,
    other: User,
    root: Uuid,
    child: Uuid,
    bookmark: Uuid,
    secret: Uuid,
    share: Uuid,
    comment: Uuid,
}

async fn category(pool: &PgPool, owner: Uuid, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner,
        CreateCategory {
            name: name.into(),
            parent_id,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("category")
    .id
}

async fn bookmark_in(pool: &PgPool, owner: Uuid, url: &str, category: Uuid) -> Uuid {
    db::create_bookmark(
        pool,
        owner,
        CreateBookmark {
            url: url.into(),
            category_ids: vec![category],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn exec(pool: &PgPool, sql: &str, id: Uuid) {
    sqlx::query(sql).bind(id).execute(pool).await.expect("exec");
}

/// A banned owner with a public, shared folder, a subfolder, and one comment.
async fn exposed() -> Option<Exposed> {
    let pool = pool_or_skip().await?;
    let admin = user(&pool, "actor").await;
    let owner = user(&pool, "owner").await;
    let other = user(&pool, "other").await;
    let root = category(&pool, owner.id, "Root", None).await;
    let child = category(&pool, owner.id, "Child", Some(root)).await;
    let bookmark = bookmark_in(&pool, owner.id, "https://seen.example/", child).await;
    let secret = bookmark_in(&pool, owner.id, "https://secret.example/", child).await;
    exec(
        &pool,
        "UPDATE bookmarks SET is_private = TRUE WHERE id = $1",
        secret,
    )
    .await;
    exec(
        &pool,
        "UPDATE categories SET visibility = 'members', public_slug = 'ldl_' || $1::text WHERE id = $1",
        root,
    )
    .await;
    exec(
        &pool,
        "UPDATE users SET profile_visibility = 'members' WHERE id = $1",
        owner.id,
    )
    .await;
    let share: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO category_shares (category_id, invitee_user_id, access, invited_by_user_id)
        VALUES ($1, $2, 'read', $3)
        RETURNING id
        "#,
    )
    .bind(root)
    .bind(other.id)
    .bind(owner.id)
    .fetch_one(&pool)
    .await
    .expect("share");
    let comment: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO social_comments (user_id, subject_type, subject_id, body)
        VALUES ($1, 'category', $2, 'rude words')
        RETURNING id
        "#,
    )
    .bind(owner.id)
    .bind(root)
    .fetch_one(&pool)
    .await
    .expect("comment");
    db::ban_user(&pool, admin.id, owner.id, &ban(None))
        .await
        .expect("ban");
    Some(Exposed {
        pool,
        admin,
        owner,
        other,
        root,
        child,
        bookmark,
        secret,
        share,
        comment,
    })
}

fn action(action: &str, subject_type: Option<&str>, subject_id: Uuid) -> ModerateAction {
    ModerateAction {
        action: action.into(),
        subject_type: subject_type.map(String::from),
        subject_id,
    }
}

async fn moderate(
    s: &Exposed,
    actions: Vec<ModerateAction>,
) -> linkdean_core::Result<linkdean_core::ModerateOutcome> {
    let req = ModerateRequest {
        reason: "Policy breach".into(),
        actions,
    };
    db::moderate_content(&s.pool, s.admin.id, s.owner.id, &req).await
}

#[tokio::test]
async fn exposed_content_lists_only_what_others_can_see() {
    let Some(s) = exposed().await else {
        return;
    };
    let c = db::exposed_content(&s.pool, s.owner.id)
        .await
        .expect("content");
    assert_eq!(c.public_links.len(), 1);
    assert_eq!(c.categories.len(), 1);
    assert_eq!(c.profile_visibility.as_deref(), Some("members"));
    assert_eq!(c.shares.len(), 1);
    assert_eq!(c.comments.len(), 1);
    let ids: Vec<Uuid> = c.bookmarks.iter().map(|b| b.id).collect();
    assert_eq!(ids, [s.bookmark], "subfolder included, private left out");
    assert!(!ids.contains(&s.secret));
}

#[tokio::test]
async fn exposed_content_excludes_vault_rows_and_trash() {
    let Some(s) = exposed().await else {
        return;
    };
    exec(
        &s.pool,
        "UPDATE bookmarks SET deleted_at = now() WHERE id = $1",
        s.bookmark,
    )
    .await;
    let (vault, key, _) = db::create_vault(&s.pool, s.owner.id, "Vault", "vault-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    let hidden = db::create_bookmark(
        &s.pool,
        s.owner.id,
        CreateBookmark {
            url: "https://vault.example/".into(),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &keyring,
    )
    .await
    .expect("vault bookmark");
    let c = db::exposed_content(&s.pool, s.owner.id)
        .await
        .expect("content");
    assert!(c.bookmarks.is_empty());

    let refused = moderate(&s, vec![action("bookmark_private", None, hidden.id)]).await;
    assert!(matches!(refused, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn the_tools_need_an_active_ban() {
    let Some(s) = exposed().await else {
        return;
    };
    let lift = LiftBans {
        user_ids: vec![s.owner.id],
    };
    db::lift_bans(&s.pool, s.admin.id, &lift)
        .await
        .expect("lift");
    let read = db::exposed_content(&s.pool, s.owner.id).await;
    assert!(matches!(read, Err(Error::Conflict(_))));
    let write = moderate(&s, vec![action("category_private", None, s.root)]).await;
    assert!(matches!(write, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn every_action_applies_is_audited_and_notifies_once() {
    let Some(s) = exposed().await else {
        return;
    };
    let actions = vec![
        action("public_link_disabled", None, s.root),
        action("category_private", None, s.root),
        action("profile_hidden", None, s.owner.id),
        action("share_revoked", Some("category_share"), s.share),
        action("comment_deleted", None, s.comment),
        action("bookmark_private", None, s.bookmark),
        action("removed_to_trash", Some("category"), s.root),
    ];
    let done = moderate(&s, actions.clone()).await.expect("moderate");
    assert_eq!((done.applied, done.skipped), (7, 0));

    let c = db::exposed_content(&s.pool, s.owner.id)
        .await
        .expect("content");
    assert!(c.public_links.is_empty() && c.categories.is_empty() && c.bookmarks.is_empty());
    assert!(c.shares.is_empty() && c.comments.is_empty());
    assert!(c.profile_visibility.is_none());

    let child_gone: bool =
        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM categories WHERE id = $1")
            .bind(s.child)
            .fetch_one(&s.pool)
            .await
            .expect("child");
    assert!(child_gone, "removing a category takes its subcategories");

    let audit = db::list_moderation_actions(&s.pool, s.owner.id)
        .await
        .expect("audit");
    assert_eq!(audit.iter().filter(|a| a.action != "ban").count(), 7);
    let comment = audit
        .iter()
        .find(|a| a.action == "comment_deleted")
        .expect("row");
    assert_eq!(comment.subject_title, "rude words");

    let again = moderate(&s, actions).await.expect("repeat");
    assert_eq!((again.applied, again.skipped), (0, 7));

    let told: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE recipient_user_id = $1 AND kind = 'moderation_changes'",
    )
    .bind(s.owner.id)
    .fetch_one(&s.pool)
    .await
    .expect("count");
    assert_eq!(told, 1, "a repeat with nothing to do tells nobody");

    let invitee: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE recipient_user_id = $1 AND kind = 'share_revoked'",
    )
    .bind(s.other.id)
    .fetch_one(&s.pool)
    .await
    .expect("count");
    assert_eq!(invitee, 1);
}

#[tokio::test]
async fn removing_a_bookmark_reports_it_for_the_search_index() {
    let Some(s) = exposed().await else {
        return;
    };
    let done = moderate(
        &s,
        vec![action("removed_to_trash", Some("bookmark"), s.bookmark)],
    )
    .await
    .expect("moderate");
    assert_eq!(done.trashed_bookmark_ids, [s.bookmark]);
}

#[tokio::test]
async fn a_subject_owned_by_someone_else_undoes_the_whole_request() {
    let Some(s) = exposed().await else {
        return;
    };
    let foreign = category(&s.pool, s.other.id, "Other", None).await;
    let res = moderate(
        &s,
        vec![
            action("category_private", None, s.root),
            action("category_private", None, foreign),
        ],
    )
    .await;
    assert!(matches!(res, Err(Error::NotFound)));
    let c = db::exposed_content(&s.pool, s.owner.id)
        .await
        .expect("content");
    assert_eq!(c.categories.len(), 1, "the first action rolled back");
}
