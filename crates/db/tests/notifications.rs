//! Notifications for share invite / access / revoke + shared edit.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, CreateDashboard, NotificationKind,
    PutDashboardLayout, PutDashboardSection, ShareAccess, UpdateBookmark, UpdateCategoryShare,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn accept_invites(pool: &sqlx::PgPool, invitee_id: Uuid) {
    for invite in db::list_share_invites(pool, invitee_id)
        .await
        .expect("inbox")
    {
        db::accept_share_invite(pool, invitee_id, invite.id)
            .await
            .expect("accept");
    }
}

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping notifications tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn email(prefix: &str) -> String {
    format!("{prefix}-{}@notif.test", Uuid::new_v4())
}

#[tokio::test]
async fn category_share_emits_invite_access_revoke() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let owner = db::create_user(
        &pool,
        &email("own"),
        Some("password-long"),
        Some("Own"),
        None,
        None,
    )
    .await
    .expect("owner");
    let invitee = db::create_user(
        &pool,
        &email("inv"),
        Some("password-long"),
        Some("Inv"),
        None,
        None,
    )
    .await
    .expect("invitee");

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
    .expect("invite");

    let list = db::list_notifications(&pool, invitee.id, 20)
        .await
        .expect("list");
    assert!(list.iter().any(|n| {
        n.kind == NotificationKind::ShareInvite && n.subject_id == cat.id && n.read_at.is_none()
    }));

    accept_invites(&pool, invitee.id).await;

    db::update_share(
        &pool,
        owner.id,
        cat.id,
        invitee.id,
        UpdateCategoryShare {
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("upgrade");

    let list = db::list_notifications(&pool, invitee.id, 20)
        .await
        .expect("list2");
    assert!(list
        .iter()
        .any(|n| n.kind == NotificationKind::ShareAccessChanged));

    db::delete_share(&pool, owner.id, cat.id, invitee.id)
        .await
        .expect("revoke");
    let list = db::list_notifications(&pool, invitee.id, 20)
        .await
        .expect("list3");
    assert!(list
        .iter()
        .any(|n| n.kind == NotificationKind::ShareRevoked));

    let unread = db::unread_notification_count(&pool, invitee.id)
        .await
        .expect("count");
    assert!(unread >= 3);
    let marked = db::mark_all_notifications_read(&pool, invitee.id)
        .await
        .expect("read all");
    assert!(marked >= 3);
    assert_eq!(
        db::unread_notification_count(&pool, invitee.id)
            .await
            .expect("zero"),
        0
    );
}

#[tokio::test]
async fn shared_bookmark_edit_notifies_owner() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let owner = db::create_user(
        &pool,
        &email("o2"),
        Some("password-long"),
        Some("O2"),
        None,
        None,
    )
    .await
    .expect("owner");
    let writer = db::create_user(
        &pool,
        &email("w2"),
        Some("password-long"),
        Some("W2"),
        None,
        None,
    )
    .await
    .expect("writer");

    let cat = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: format!("Edit-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("cat");

    db::create_share(
        &pool,
        owner.id,
        cat.id,
        CreateCategoryShare {
            email: writer.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("share");
    accept_invites(&pool, writer.id).await;

    let bm = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: format!("https://notif.example/{}/", Uuid::new_v4()),
            title: Some("Owned".into()),
            description: None,
            notes: None,
            category_id: Some(cat.id),
            category_ids: vec![cat.id],
            tags: vec![],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bm");

    // Clear invite noise for owner unread focus
    let _ = db::mark_all_notifications_read(&pool, owner.id).await;

    db::update_bookmark(
        &pool,
        writer.id,
        bm.id,
        UpdateBookmark {
            title: Some("Edited by invitee".into()),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("edit");

    let list = db::list_notifications(&pool, owner.id, 20)
        .await
        .expect("owner notifs");
    assert!(list.iter().any(|n| {
        n.kind == NotificationKind::SharedEdit && n.subject_id == bm.id && n.read_at.is_none()
    }));
}

#[tokio::test]
async fn shared_dashboard_edit_notifies_owner() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let owner = db::create_user(
        &pool,
        &email("o3"),
        Some("password-long"),
        Some("O3"),
        None,
        None,
    )
    .await
    .expect("owner");
    let writer = db::create_user(
        &pool,
        &email("w3"),
        Some("password-long"),
        Some("W3"),
        None,
        None,
    )
    .await
    .expect("writer");

    let home = db::create_dashboard(
        &pool,
        owner.id,
        CreateDashboard {
            title: Some(format!("Board-{}", Uuid::new_v4())),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dash");

    db::create_dashboard_share(
        &pool,
        owner.id,
        home.id,
        linkdean_core::CreateDashboardShare {
            email: writer.email.clone(),
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("share");
    accept_invites(&pool, writer.id).await;

    let _ = db::mark_all_notifications_read(&pool, owner.id).await;

    db::put_dashboard(
        &pool,
        writer.id,
        home.id,
        PutDashboardLayout {
            title: Some("RW edit".into()),
            sections: vec![PutDashboardSection {
                title: "Pins".into(),
                bookmark_ids: vec![],
            }],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("put");

    let list = db::list_notifications(&pool, owner.id, 20)
        .await
        .expect("list");
    assert!(list
        .iter()
        .any(|n| { n.kind == NotificationKind::SharedEdit && n.subject_id == home.id }));
}
