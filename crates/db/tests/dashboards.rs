//! Personal dashboard (M2) + shared dashboards (M3) repo tests.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
use linkdean_core::{
    CategoryAccess, CreateBookmark, CreateCategory, CreateDashboard, CreateDashboardShare,
    CreateDashboardShareOutcome, Error, PatchDashboard, PutDashboardLayout, PutDashboardSection,
    ShareAccess, UpdateDashboardShare,
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
        eprintln!("skipping dashboard tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url)
        .await
        .expect("connect postgres for dashboard tests");
    db::migrate(&pool)
        .await
        .expect("migrate for dashboard tests");
    Some(pool)
}

fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@dash.test", Uuid::new_v4())
}

#[tokio::test]
async fn default_dashboard_roundtrip_and_authz() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("a"),
        Some("password-a-long"),
        Some("Dash A"),
        None,
        None,
    )
    .await
    .expect("create user a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");
    let user_b = db::create_user(
        &pool,
        &unique_email("b"),
        Some("password-b-long"),
        Some("Dash B"),
        None,
        None,
    )
    .await
    .expect("create user b");
    db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("DashCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("create category");

    let bm1 = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://dash.example/{}/1", Uuid::new_v4()),
            title: Some("One".into()),
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
    .expect("bm1");
    let bm2 = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://dash.example/{}/2", Uuid::new_v4()),
            title: Some("Two".into()),
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
    .expect("bm2");

    let empty = db::get_default_dashboard(&pool, user_a.id, &VaultKeyring::default())
        .await
        .expect("ensure default");
    assert!(empty.is_default);
    assert_eq!(empty.title, "Home");
    assert!(empty.sections.is_empty());

    let saved = db::put_default_dashboard(
        &pool,
        user_a.id,
        PutDashboardLayout {
            title: Some("My desk".into()),
            sections: vec![
                PutDashboardSection {
                    title: "Later".into(),
                    bookmark_ids: vec![bm2.id],
                },
                PutDashboardSection {
                    title: "Now".into(),
                    bookmark_ids: vec![bm1.id, bm2.id],
                },
            ],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("put layout");

    assert_eq!(saved.title, "My desk");
    assert_eq!(saved.sections.len(), 2);
    assert_eq!(saved.sections[0].title, "Later");
    assert_eq!(saved.sections[0].items.len(), 1);
    assert_eq!(saved.sections[0].items[0].bookmark.id, bm2.id);
    assert_eq!(saved.sections[1].title, "Now");
    assert_eq!(saved.sections[1].items.len(), 2);
    assert_eq!(
        saved.sections[1]
            .items
            .iter()
            .map(|i| i.bookmark.id)
            .collect::<Vec<_>>(),
        vec![bm1.id, bm2.id]
    );

    // B cannot pin A's bookmark onto B's dashboard
    let denied = db::put_default_dashboard(
        &pool,
        user_b.id,
        PutDashboardLayout {
            title: None,
            sections: vec![PutDashboardSection {
                title: "Stolen".into(),
                bookmark_ids: vec![bm1.id],
            }],
        },
        &VaultKeyring::default(),
    )
    .await;
    assert!(matches!(denied, Err(Error::BadRequest(_))));

    // Delete bm2 → GET skips it; section still listed
    db::delete_bookmark(&pool, user_a.id, bm2.id)
        .await
        .expect("delete bm2");
    let after_delete = db::get_default_dashboard(&pool, user_a.id, &VaultKeyring::default())
        .await
        .expect("get after delete");
    assert_eq!(after_delete.sections.len(), 2);
    assert!(after_delete.sections[0].items.is_empty());
    assert_eq!(after_delete.sections[1].items.len(), 1);
    assert_eq!(after_delete.sections[1].items[0].bookmark.id, bm1.id);

    let _ = db::delete_bookmark(&pool, user_a.id, bm1.id).await;
    let _ = db::delete_category(&pool, user_a.id, cat.id).await;
}

#[tokio::test]
async fn multi_dashboard_create_pin_delete() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user = db::create_user(
        &pool,
        &unique_email("multi"),
        Some("password-a-long"),
        Some("Multi"),
        None,
        None,
    )
    .await
    .expect("create user");
    db::mark_email_verified(&pool, user.id)
        .await
        .expect("verify email");

    let home = db::get_default_dashboard(&pool, user.id, &VaultKeyring::default())
        .await
        .expect("default");
    assert!(home.is_default);

    let second = db::create_dashboard(
        &pool,
        user.id,
        CreateDashboard {
            title: Some("Work".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("create second");
    assert!(!second.is_default);

    let listed = db::list_dashboards(&pool, user.id).await.expect("list");
    assert_eq!(listed.len(), 2);

    let pinned = db::patch_dashboard(
        &pool,
        user.id,
        second.id,
        PatchDashboard {
            title: None,
            is_default: Some(true),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("pin second");
    assert!(pinned.is_default);

    let default_now = db::get_default_dashboard(&pool, user.id, &VaultKeyring::default())
        .await
        .expect("default after pin");
    assert_eq!(default_now.id, second.id);

    let after_del = db::delete_dashboard(&pool, user.id, second.id)
        .await
        .expect("delete pin");
    assert_eq!(after_del.len(), 1);
    assert!(after_del[0].is_default);
    assert_eq!(after_del[0].id, home.id);
}

#[tokio::test]
async fn dashboard_share_read_write_and_revoke() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user_a = db::create_user(
        &pool,
        &unique_email("share-a"),
        Some("password-a-long"),
        Some("Dash Share A"),
        None,
        None,
    )
    .await
    .expect("create a");
    db::mark_email_verified(&pool, user_a.id)
        .await
        .expect("verify email");
    let user_b = db::create_user(
        &pool,
        &unique_email("share-b"),
        Some("password-b-long"),
        Some("Dash Share B"),
        None,
        None,
    )
    .await
    .expect("create b");
    db::mark_email_verified(&pool, user_b.id)
        .await
        .expect("verify email");
    let user_c = db::create_user(
        &pool,
        &unique_email("share-c"),
        Some("password-c-long"),
        Some("Dash Share C"),
        None,
        None,
    )
    .await
    .expect("create c");
    db::mark_email_verified(&pool, user_c.id)
        .await
        .expect("verify email");

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("DashShareCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("cat");
    let bm = db::create_bookmark(
        &pool,
        user_a.id,
        CreateBookmark {
            url: format!("https://dash-share.example/{}/", Uuid::new_v4()),
            title: Some("Pinned".into()),
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

    let home = db::get_default_dashboard(&pool, user_a.id, &VaultKeyring::default())
        .await
        .expect("home");
    let _ = db::put_dashboard(
        &pool,
        user_a.id,
        home.id,
        PutDashboardLayout {
            title: Some("Team board".into()),
            sections: vec![PutDashboardSection {
                title: "Pins".into(),
                bookmark_ids: vec![bm.id],
            }],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("layout");

    assert!(matches!(
        db::get_dashboard(&pool, user_c.id, home.id, &VaultKeyring::default()).await,
        Err(Error::NotFound)
    ));

    let outcome = db::create_dashboard_share(
        &pool,
        user_a.id,
        home.id,
        CreateDashboardShare {
            email: user_b.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("share read");
    assert!(matches!(
        outcome,
        CreateDashboardShareOutcome::Pending { .. }
    ));
    accept_invites(&pool, user_b.id).await;

    let as_b = db::get_dashboard(&pool, user_b.id, home.id, &VaultKeyring::default())
        .await
        .expect("b read");
    assert_eq!(as_b.access, CategoryAccess::Read);
    assert_eq!(as_b.sections[0].items.len(), 1);
    assert!(!as_b.is_default);

    assert!(matches!(
        db::put_dashboard(
            &pool,
            user_b.id,
            home.id,
            PutDashboardLayout {
                title: None,
                sections: vec![PutDashboardSection {
                    title: "Nope".into(),
                    bookmark_ids: vec![],
                }],
            },
            &VaultKeyring::default(),
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        db::delete_dashboard(&pool, user_b.id, home.id).await,
        Err(Error::Forbidden)
    ));

    let _ = db::update_dashboard_share(
        &pool,
        user_a.id,
        home.id,
        user_b.id,
        UpdateDashboardShare {
            access: ShareAccess::Write,
        },
    )
    .await
    .expect("upgrade write");

    let rewritten = db::put_dashboard(
        &pool,
        user_b.id,
        home.id,
        PutDashboardLayout {
            title: Some("Team board RW".into()),
            sections: vec![PutDashboardSection {
                title: "Pins".into(),
                bookmark_ids: vec![bm.id],
            }],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("b write layout");
    assert_eq!(rewritten.access, CategoryAccess::Write);
    assert_eq!(rewritten.title, "Team board RW");

    let listed_b = db::list_dashboards(&pool, user_b.id).await.expect("list b");
    assert!(listed_b
        .iter()
        .any(|d| d.id == home.id && d.access == CategoryAccess::Write));

    db::delete_dashboard_share(&pool, user_a.id, home.id, user_b.id)
        .await
        .expect("revoke");
    assert!(matches!(
        db::get_dashboard(&pool, user_b.id, home.id, &VaultKeyring::default()).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn saving_a_layout_keeps_pins_the_editor_cannot_see() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = db::create_user(
        &pool,
        &unique_email("hidden"),
        Some("password-long"),
        None,
        None,
        None,
    )
    .await
    .expect("user");
    db::mark_email_verified(&pool, owner.id)
        .await
        .expect("verify email");
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Work", "work-passphrase")
        .await
        .expect("vault");
    let mut unlocked = VaultKeyring::default();
    unlocked.insert(vault.id, key);

    let plain = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: format!("https://plain.example/{}", Uuid::new_v4()),
            ..Default::default()
        },
        &unlocked,
    )
    .await
    .expect("plain")
    .id;
    let trashed = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: format!("https://trashed.example/{}", Uuid::new_v4()),
            ..Default::default()
        },
        &unlocked,
    )
    .await
    .expect("trashed")
    .id;
    let secret = db::create_bookmark(
        &pool,
        owner.id,
        CreateBookmark {
            url: "https://secret.example/".into(),
            vault_id: Some(vault.id),
            ..Default::default()
        },
        &unlocked,
    )
    .await
    .expect("vault bookmark")
    .id;

    let board = db::create_dashboard(
        &pool,
        owner.id,
        CreateDashboard {
            title: Some("Board".into()),
        },
        &unlocked,
    )
    .await
    .expect("dashboard");
    db::put_dashboard(
        &pool,
        owner.id,
        board.id,
        PutDashboardLayout {
            title: None,
            sections: vec![PutDashboardSection {
                title: "Main".into(),
                bookmark_ids: vec![plain, trashed, secret],
            }],
        },
        &unlocked,
    )
    .await
    .expect("first layout");
    db::delete_bookmark(&pool, owner.id, trashed)
        .await
        .expect("trash");

    // Saved while the vault is locked: the editor sees only `plain`, renamed into a new section.
    db::put_dashboard(
        &pool,
        owner.id,
        board.id,
        PutDashboardLayout {
            title: None,
            sections: vec![PutDashboardSection {
                title: "Renamed".into(),
                bookmark_ids: vec![plain],
            }],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("locked layout");

    let mut kept: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT i.bookmark_id FROM dashboard_items i
        JOIN dashboard_sections s ON s.id = i.section_id
        WHERE s.dashboard_id = $1
        "#,
    )
    .bind(board.id)
    .fetch_all(&pool)
    .await
    .expect("items");
    kept.sort();
    let mut want = vec![plain, trashed, secret];
    want.sort();
    assert_eq!(kept, want);

    // Removing every section still keeps the hidden pins.
    db::put_dashboard(
        &pool,
        owner.id,
        board.id,
        PutDashboardLayout {
            title: None,
            sections: vec![],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("empty layout");
    let left: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM dashboard_items i
        JOIN dashboard_sections s ON s.id = i.section_id
        WHERE s.dashboard_id = $1
        "#,
    )
    .bind(board.id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(left, 2);
}
