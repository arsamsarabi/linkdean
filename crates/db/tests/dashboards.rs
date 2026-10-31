//! Personal dashboard (M2) repo tests.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateDashboard, Error, PatchDashboard, PutDashboardLayout,
    PutDashboardSection,
};
use linkdean_db::{self as db};
use uuid::Uuid;

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

    let cat = db::create_category(
        &pool,
        user_a.id,
        CreateCategory {
            name: format!("DashCat-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
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
        },
        None,
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
        },
        None,
    )
    .await
    .expect("bm2");

    let empty = db::get_default_dashboard(&pool, user_a.id, None)
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
        None,
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
        None,
    )
    .await;
    assert!(matches!(denied, Err(Error::BadRequest(_))));

    // Delete bm2 → GET skips it; section still listed
    db::delete_bookmark(&pool, user_a.id, bm2.id)
        .await
        .expect("delete bm2");
    let after_delete = db::get_default_dashboard(&pool, user_a.id, None)
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

    let home = db::get_default_dashboard(&pool, user.id, None)
        .await
        .expect("default");
    assert!(home.is_default);

    let second = db::create_dashboard(
        &pool,
        user.id,
        CreateDashboard {
            title: Some("Work".into()),
        },
        None,
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
        None,
    )
    .await
    .expect("pin second");
    assert!(pinned.is_default);

    let default_now = db::get_default_dashboard(&pool, user.id, None)
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
