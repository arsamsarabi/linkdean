//! A Linkdean JSON export restores into another account with folders, flags, times and
//! dashboards.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::import_export::{build_linkdean_export, parse_linkdean_json};
use linkdean_core::{
    CreateBookmark, CreateCategory, CreateDashboard, PutDashboardLayout, PutDashboardSection, User,
    VaultKeyring,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping export tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@export.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn category(pool: &PgPool, owner: &User, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
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

#[tokio::test]
async fn json_export_restores_folders_flags_times_and_dashboards() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let from = person(&pool, "from").await;
    let work = category(&pool, &from, "Work", None).await;
    let sub = category(&pool, &from, "Sub", Some(work)).await;
    let url = format!("https://export.example/{}", Uuid::new_v4());
    let bookmark = db::create_bookmark(
        &pool,
        from.id,
        CreateBookmark {
            url: url.clone(),
            title: Some("Exported".into()),
            category_ids: vec![work, sub],
            is_favorite: true,
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id;
    sqlx::query(
        "UPDATE bookmarks SET is_private = TRUE, created_at = '2020-01-02T03:04:05Z' WHERE id = $1",
    )
    .bind(bookmark)
    .execute(&pool)
    .await
    .expect("seed");
    let board = db::create_dashboard(
        &pool,
        from.id,
        CreateDashboard {
            title: Some("Reading".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard");
    db::put_dashboard(
        &pool,
        from.id,
        board.id,
        PutDashboardLayout {
            title: None,
            sections: vec![PutDashboardSection {
                title: "Top".into(),
                bookmark_ids: vec![bookmark],
            }],
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("layout");

    let (cats, bms, dashboards) = db::export_data(&pool, from.id).await.expect("export");
    let raw = serde_json::to_string(&build_linkdean_export(cats, bms, dashboards)).expect("json");

    let to = person(&pool, "to").await;
    let parsed = parse_linkdean_json(&raw).expect("parse");
    let (_, imported, failed) = db::apply_parsed_import(&pool, to.id, parsed)
        .await
        .expect("import");
    assert_eq!((imported, failed), (1, 0));

    let (id, is_private, created): (Uuid, bool, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        "SELECT id, is_private, created_at FROM bookmarks WHERE owner_user_id = $1 AND url = $2",
    )
    .bind(to.id)
    .bind(&url)
    .fetch_one(&pool)
    .await
    .expect("restored");
    assert!(is_private);
    assert_eq!(created.to_rfc3339(), "2020-01-02T03:04:05+00:00");

    let raw_row = db::get_bookmark_raw(&pool, to.id, id).await.expect("raw");
    let view = db::bookmark_to_view(&pool, raw_row, &VaultKeyring::default(), to.id)
        .await
        .expect("view")
        .expect("visible");
    let mut paths: Vec<String> = view.categories.iter().map(|c| c.path.clone()).collect();
    paths.sort();
    assert_eq!(paths, vec!["Work".to_string(), "Work / Sub".to_string()]);
    assert!(view.is_favorite);

    let pinned: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT i.bookmark_id FROM dashboard_items i
        JOIN dashboard_sections s ON s.id = i.section_id
        JOIN dashboards d ON d.id = s.dashboard_id
        WHERE d.owner_user_id = $1 AND d.title = 'Reading' AND s.title = 'Top'
        "#,
    )
    .bind(to.id)
    .fetch_all(&pool)
    .await
    .expect("pins");
    assert_eq!(pinned, vec![id]);
}

async fn folder_tree(pool: &PgPool, owner: &User) -> Vec<(String, Option<String>)> {
    let mut rows: Vec<(String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT c.name, p.name FROM categories c
        LEFT JOIN categories p ON p.id = c.parent_id
        WHERE c.owner_user_id = $1
        "#,
    )
    .bind(owner.id)
    .fetch_all(pool)
    .await
    .expect("tree");
    rows.sort();
    rows
}

#[tokio::test]
async fn html_import_keeps_slashes_and_same_name_folders_apart() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let to = person(&pool, "slash").await;
    let html = "<DL><p>\n<DT><H3>AC/DC</H3>\n<DL><p>\n<DT><H3>Live</H3>\n<DL><p>\n<DT><A HREF=\"https://a.example\">A</A>\n</DL><p>\n</DL><p>\n<DT><H3>Rock</H3>\n<DL><p>\n<DT><H3>Live</H3>\n<DL><p>\n<DT><A HREF=\"https://b.example\">B</A>\n</DL><p>\n</DL><p>\n</DL><p>\n";
    let parsed = linkdean_core::import_export::parse_netscape(html).expect("parse");
    db::apply_parsed_import(&pool, to.id, parsed)
        .await
        .expect("import");
    assert_eq!(
        folder_tree(&pool, &to).await,
        vec![
            ("AC/DC".to_string(), None),
            ("Live".to_string(), Some("AC/DC".to_string())),
            ("Live".to_string(), Some("Rock".to_string())),
            ("Rock".to_string(), None),
        ]
    );
}

#[tokio::test]
async fn a_failed_import_reports_what_it_got_through() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let to = person(&pool, "partial").await;
    // The dashboard's blank section title fails after the bookmark is in.
    let raw = r#"{"version": 2, "exported_at": "2026-01-01T00:00:00Z", "categories": [],
        "bookmarks": [{"id": "00000000-0000-0000-0000-000000000001", "url": "https://p.example", "title": "P", "tags": [], "is_vault": false}],
        "dashboards": [{"title": "D", "is_default": false, "sections": [{"title": " ", "bookmark_ids": []}]}]}"#;
    let parsed = parse_linkdean_json(raw).expect("parse");
    let mut counts = db::ImportCounts::default();
    let result = db::apply_parsed_import_counted(&pool, to.id, parsed, &mut counts).await;
    assert!(result.is_err());
    assert_eq!((counts.total, counts.imported, counts.failed), (1, 1, 0));
}

#[tokio::test]
async fn import_skips_links_that_are_not_web_addresses() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let to = person(&pool, "scheme").await;
    let html = "<DL><p>\n<DT><A HREF=\"https://ok.example\">OK</A>\n<DT><A HREF=\"javascript:alert(1)\">Bad</A>\n</DL><p>\n";
    let parsed = linkdean_core::import_export::parse_netscape(html).expect("parse");
    db::apply_parsed_import(&pool, to.id, parsed)
        .await
        .expect("import");
    let urls: Vec<(String,)> = sqlx::query_as("SELECT url FROM bookmarks WHERE owner_user_id = $1")
        .bind(to.id)
        .fetch_all(&pool)
        .await
        .expect("urls");
    assert_eq!(urls, vec![("https://ok.example".to_string(),)]);
}
