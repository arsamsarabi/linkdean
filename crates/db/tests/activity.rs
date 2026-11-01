//! Personal activity history for create / update / delete / restore.
//! Requires `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    ActivityAction, ActivitySubjectType, CreateBookmark, CreateCategory, CreateDashboard,
    ListActivityFilter, UpdateBookmark, UpdateCategory,
};
use linkdean_db::{self as db};
use uuid::Uuid;

async fn pool_or_skip() -> Option<sqlx::PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping activity tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

fn email(prefix: &str) -> String {
    format!("{prefix}-{}@activity.test", Uuid::new_v4())
}

#[tokio::test]
async fn bookmark_category_dashboard_activity_flow() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };

    let user = db::create_user(
        &pool,
        &email("u"),
        Some("password-long"),
        Some("Act"),
        None,
        None,
    )
    .await
    .expect("user");

    let cat = db::create_category(
        &pool,
        user.id,
        CreateCategory {
            name: format!("Folder-{}", Uuid::new_v4()),
            parent_id: None,
            is_hidden: false,
            is_vault: false,
            icon: None,
        },
    )
    .await
    .expect("cat");

    let bm = db::create_bookmark(
        &pool,
        user.id,
        CreateBookmark {
            url: "https://example.com/activity".into(),
            title: Some("Activity BM".into()),
            description: None,
            notes: None,
            category_id: Some(cat.id),
            category_ids: vec![],
            tags: vec![],
            is_favorite: false,
            is_unread: false,
            is_pinned: false,
        },
        None,
    )
    .await
    .expect("bm");

    db::update_bookmark(
        &pool,
        user.id,
        bm.id,
        UpdateBookmark {
            url: None,
            title: Some("Activity BM Renamed".into()),
            description: None,
            notes: None,
            category_id: None,
            category_ids: None,
            tags: None,
            is_favorite: None,
            is_unread: None,
            is_pinned: None,
        },
        None,
    )
    .await
    .expect("update bm");

    let renamed = format!("Folder-renamed-{}", Uuid::new_v4());
    db::update_category(
        &pool,
        user.id,
        cat.id,
        UpdateCategory {
            name: Some(renamed.clone()),
            parent_id: None,
            is_hidden: None,
            position: None,
            icon: None,
        },
    )
    .await
    .expect("update cat");

    let dash = db::create_dashboard(
        &pool,
        user.id,
        CreateDashboard {
            title: Some("Board".into()),
        },
        None,
    )
    .await
    .expect("dash");

    db::delete_bookmark(&pool, user.id, bm.id)
        .await
        .expect("delete bm");
    db::restore_bookmark(&pool, user.id, bm.id)
        .await
        .expect("restore bm");

    let all = db::list_activity(
        &pool,
        user.id,
        ListActivityFilter {
            subject_type: None,
            action: None,
            limit: 50,
        },
    )
    .await
    .expect("list");

    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Created
            && e.subject_type == ActivitySubjectType::Bookmark
            && e.subject_id == bm.id
    }));
    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Updated
            && e.subject_type == ActivitySubjectType::Bookmark
            && e.subject_id == bm.id
            && e.summary.contains("renamed")
            && e.summary.contains("Activity BM Renamed")
    }));
    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Updated
            && e.subject_type == ActivitySubjectType::Category
            && e.subject_id == cat.id
            && e.summary.contains("renamed")
            && e.summary.contains(&renamed)
    }));
    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Deleted
            && e.subject_type == ActivitySubjectType::Bookmark
            && e.subject_id == bm.id
    }));
    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Restored
            && e.subject_type == ActivitySubjectType::Bookmark
            && e.subject_id == bm.id
    }));
    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Created
            && e.subject_type == ActivitySubjectType::Category
            && e.subject_id == cat.id
    }));
    assert!(all.iter().any(|e| {
        e.action == ActivityAction::Created
            && e.subject_type == ActivitySubjectType::Dashboard
            && e.subject_id == dash.id
    }));

    let bookmarks_only = db::list_activity(
        &pool,
        user.id,
        ListActivityFilter {
            subject_type: Some(ActivitySubjectType::Bookmark),
            action: Some(ActivityAction::Updated),
            limit: 20,
        },
    )
    .await
    .expect("filter");
    assert!(!bookmarks_only.is_empty());
    assert!(bookmarks_only.iter().all(|e| {
        e.subject_type == ActivitySubjectType::Bookmark && e.action == ActivityAction::Updated
    }));
}
