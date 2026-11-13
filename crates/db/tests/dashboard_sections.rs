//! Sections that fill themselves, display modes, and bookmark opens.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::VaultKeyring;
use linkdean_core::{
    CreateBookmark, CreateCategory, CreateDashboard, CreateDashboardShare, DashboardOpenWindow,
    DashboardSectionDisplay, DashboardSectionKind, DashboardSectionSettings,
    DashboardSectionSource, DashboardView, Error, PutDashboardLayout, PutDashboardSection,
    ShareAccess, User,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping dashboard section tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@sections.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    db::mark_email_verified(pool, user.id)
        .await
        .expect("verify email");
    user
}

async fn board(pool: &PgPool, owner: &User) -> Uuid {
    db::create_dashboard(
        pool,
        owner.id,
        CreateDashboard {
            title: Some("Mine".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard")
    .id
}

async fn bookmark(pool: &PgPool, owner: &User, req: CreateBookmark) -> Uuid {
    db::create_bookmark(pool, owner.id, req, &VaultKeyring::default())
        .await
        .expect("bookmark")
        .id
}

fn at(url: &str) -> CreateBookmark {
    CreateBookmark {
        url: url.into(),
        title: Some(url.into()),
        ..Default::default()
    }
}

fn section(title: &str, settings: DashboardSectionSettings) -> PutDashboardSection {
    PutDashboardSection {
        title: title.into(),
        bookmark_ids: Vec::new(),
        settings,
    }
}

fn latest(source: DashboardSectionSource) -> DashboardSectionSettings {
    DashboardSectionSettings {
        kind: DashboardSectionKind::Latest,
        source,
        ..Default::default()
    }
}

async fn put(
    pool: &PgPool,
    owner: &User,
    id: Uuid,
    sections: Vec<PutDashboardSection>,
) -> Result<DashboardView, Error> {
    db::put_dashboard(
        pool,
        owner.id,
        id,
        PutDashboardLayout {
            title: None,
            sections,
        },
        &VaultKeyring::default(),
    )
    .await
}

fn urls(view: &DashboardView, index: usize) -> Vec<String> {
    view.sections[index]
        .items
        .iter()
        .map(|i| i.bookmark.url.clone())
        .collect()
}

#[tokio::test]
async fn latest_matches_every_source_filter_newest_first() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "latest").await;
    let other = user(&pool, "other").await;
    let work = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Work".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder");
    let sub = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Rust".into(),
            parent_id: Some(work.id),
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("subfolder");

    bookmark(&pool, &owner, at("https://example.org/loose")).await;
    bookmark(
        &pool,
        &owner,
        CreateBookmark {
            category_ids: vec![work.id],
            tags: vec!["rust".into()],
            ..at("https://www.github.com/a")
        },
    )
    .await;
    bookmark(
        &pool,
        &owner,
        CreateBookmark {
            category_ids: vec![sub.id],
            is_favorite: true,
            ..at("https://docs.github.com/b")
        },
    )
    .await;
    bookmark(
        &pool,
        &owner,
        CreateBookmark {
            category_ids: vec![sub.id],
            is_unread: true,
            tags: vec!["Rust".into()],
            ..at("https://notgithub.com/c")
        },
    )
    .await;
    let trashed = bookmark(
        &pool,
        &owner,
        CreateBookmark {
            category_ids: vec![work.id],
            ..at("https://github.com/trashed")
        },
    )
    .await;
    db::delete_bookmark(&pool, owner.id, trashed)
        .await
        .expect("trash");
    // Someone else's bookmark never shows on this owner's dashboard.
    bookmark(&pool, &other, at("https://github.com/theirs")).await;

    let id = board(&pool, &owner).await;
    let view = put(
        &pool,
        &owner,
        id,
        vec![
            section("All", latest(DashboardSectionSource::default())),
            section(
                "Work",
                latest(DashboardSectionSource {
                    category_id: Some(work.id),
                    ..Default::default()
                }),
            ),
            section(
                "Tag",
                latest(DashboardSectionSource {
                    tag: Some("RUST".into()),
                    ..Default::default()
                }),
            ),
            section(
                "Site",
                latest(DashboardSectionSource {
                    site: Some("https://www.GitHub.com/".into()),
                    ..Default::default()
                }),
            ),
            section(
                "Favorites in Work",
                latest(DashboardSectionSource {
                    category_id: Some(work.id),
                    favorites: true,
                    ..Default::default()
                }),
            ),
            section(
                "Unread",
                latest(DashboardSectionSource {
                    unread: true,
                    ..Default::default()
                }),
            ),
        ],
    )
    .await
    .expect("layout");

    assert_eq!(
        urls(&view, 0),
        [
            "https://notgithub.com/c",
            "https://docs.github.com/b",
            "https://www.github.com/a",
            "https://example.org/loose",
        ]
    );
    assert_eq!(
        urls(&view, 1),
        [
            "https://notgithub.com/c",
            "https://docs.github.com/b",
            "https://www.github.com/a",
        ]
    );
    assert_eq!(
        urls(&view, 2),
        ["https://notgithub.com/c", "https://www.github.com/a"]
    );
    assert_eq!(
        urls(&view, 3),
        ["https://docs.github.com/b", "https://www.github.com/a"]
    );
    assert_eq!(
        view.sections[3].settings.source.site.as_deref(),
        Some("github.com")
    );
    assert_eq!(urls(&view, 4), ["https://docs.github.com/b"]);
    assert_eq!(urls(&view, 5), ["https://notgithub.com/c"]);
}

#[tokio::test]
async fn a_section_shows_at_most_its_limit_and_rejects_limits_out_of_range() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "limit").await;
    for i in 0..6 {
        bookmark(&pool, &owner, at(&format!("https://limit.example/{i}"))).await;
    }
    let id = board(&pool, &owner).await;
    let view = put(
        &pool,
        &owner,
        id,
        vec![section(
            "Four",
            DashboardSectionSettings {
                item_limit: 4,
                display: DashboardSectionDisplay::Icons,
                ..latest(DashboardSectionSource::default())
            },
        )],
    )
    .await
    .expect("layout");
    assert_eq!(view.sections[0].items.len(), 4);
    assert_eq!(
        view.sections[0].settings.display,
        DashboardSectionDisplay::Icons
    );

    for bad in [3, 25] {
        let err = put(
            &pool,
            &owner,
            id,
            vec![section(
                "Bad",
                DashboardSectionSettings {
                    item_limit: bad,
                    ..latest(DashboardSectionSource::default())
                },
            )],
        )
        .await
        .expect_err("out of range");
        assert!(matches!(err, Error::BadRequest(_)), "{err:?}");
    }
}

#[tokio::test]
async fn most_opened_and_recently_visited_follow_the_owners_opens() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "opens").await;
    let a = bookmark(&pool, &owner, at("https://opens.example/a")).await;
    let b = bookmark(&pool, &owner, at("https://opens.example/b")).await;
    let old = bookmark(&pool, &owner, at("https://opens.example/old")).await;
    bookmark(&pool, &owner, at("https://opens.example/never")).await;

    for _ in 0..3 {
        db::record_bookmark_open(&pool, owner.id, a)
            .await
            .expect("open a");
    }
    for _ in 0..5 {
        db::record_bookmark_open(&pool, owner.id, old)
            .await
            .expect("open old");
    }
    // The old bookmark's opens were 40 days ago: they count for all time only.
    sqlx::query(
        "UPDATE bookmark_opens SET opened_at = now() - interval '40 days' WHERE user_id = $1 AND bookmark_id = $2",
    )
    .bind(owner.id)
    .bind(old)
    .execute(&pool)
    .await
    .expect("age opens");
    sqlx::query(
        "UPDATE bookmark_open_stats SET last_opened_at = now() - interval '40 days' WHERE user_id = $1 AND bookmark_id = $2",
    )
    .bind(owner.id)
    .bind(old)
    .execute(&pool)
    .await
    .expect("age stats");
    db::record_bookmark_open(&pool, owner.id, b)
        .await
        .expect("open b");

    let opened = |window| DashboardSectionSettings {
        kind: DashboardSectionKind::MostOpened,
        open_window: window,
        ..Default::default()
    };
    let id = board(&pool, &owner).await;
    let view = put(
        &pool,
        &owner,
        id,
        vec![
            section("All time", opened(DashboardOpenWindow::AllTime)),
            section("30 days", opened(DashboardOpenWindow::Last30Days)),
            section(
                "Recent",
                DashboardSectionSettings {
                    kind: DashboardSectionKind::RecentlyVisited,
                    ..Default::default()
                },
            ),
        ],
    )
    .await
    .expect("layout");

    assert_eq!(
        urls(&view, 0),
        [
            "https://opens.example/old",
            "https://opens.example/a",
            "https://opens.example/b",
        ]
    );
    assert_eq!(
        urls(&view, 1),
        ["https://opens.example/a", "https://opens.example/b"]
    );
    assert_eq!(
        urls(&view, 2),
        [
            "https://opens.example/b",
            "https://opens.example/a",
            "https://opens.example/old",
        ]
    );

    // A new open prunes the owner's opens older than 30 days.
    db::record_bookmark_open(&pool, owner.id, a)
        .await
        .expect("open a again");
    let left: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bookmark_opens WHERE user_id = $1 AND bookmark_id = $2",
    )
    .bind(owner.id)
    .bind(old)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(left, 0);
}

#[tokio::test]
async fn opens_are_per_person_and_need_read_access() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "mine").await;
    let stranger = user(&pool, "stranger").await;
    let a = bookmark(&pool, &owner, at("https://per-person.example/a")).await;

    let err = db::record_bookmark_open(&pool, stranger.id, a)
        .await
        .expect_err("not readable");
    assert!(matches!(err, Error::NotFound), "{err:?}");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bookmark_open_stats WHERE bookmark_id = $1")
            .bind(a)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn self_filling_sections_and_sharing_exclude_each_other() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "excl").await;
    let friend = user(&pool, "friend").await;

    // A shared dashboard (or one with an invite waiting) takes pinned sections only.
    for email in [
        friend.email.clone(),
        format!("{}@nobody.test", Uuid::new_v4()),
    ] {
        let id = board(&pool, &owner).await;
        db::create_dashboard_share(
            &pool,
            owner.id,
            id,
            CreateDashboardShare {
                email,
                access: ShareAccess::Read,
            },
        )
        .await
        .expect("share");
        let err = put(
            &pool,
            &owner,
            id,
            vec![section("Latest", latest(DashboardSectionSource::default()))],
        )
        .await
        .expect_err("shared");
        assert!(matches!(err, Error::BadRequest(_)), "{err:?}");
        put(
            &pool,
            &owner,
            id,
            vec![section("Pins", DashboardSectionSettings::default())],
        )
        .await
        .expect("pinned is fine");
    }

    // A dashboard with a section that fills itself cannot be shared.
    let id = board(&pool, &owner).await;
    put(
        &pool,
        &owner,
        id,
        vec![section(
            "Recent",
            DashboardSectionSettings {
                kind: DashboardSectionKind::RecentlyVisited,
                ..Default::default()
            },
        )],
    )
    .await
    .expect("layout");
    let err = db::create_dashboard_share(
        &pool,
        owner.id,
        id,
        CreateDashboardShare {
            email: friend.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect_err("has self-filling section");
    assert!(matches!(err, Error::BadRequest(_)), "{err:?}");
}

#[tokio::test]
async fn a_folder_source_must_be_the_owners_and_a_deleted_one_empties_the_section() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "src").await;
    let other = user(&pool, "src-other").await;
    let theirs = db::create_category(
        &pool,
        other.id,
        CreateCategory {
            name: "Theirs".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder");
    bookmark(&pool, &owner, at("https://src.example/a")).await;
    let id = board(&pool, &owner).await;
    let err = put(
        &pool,
        &owner,
        id,
        vec![section(
            "Theirs",
            latest(DashboardSectionSource {
                category_id: Some(theirs.id),
                ..Default::default()
            }),
        )],
    )
    .await
    .expect_err("not mine");
    assert!(matches!(err, Error::BadRequest(_)), "{err:?}");

    let mine = db::create_category(
        &pool,
        owner.id,
        CreateCategory {
            name: "Mine".into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder");
    put(
        &pool,
        &owner,
        id,
        vec![section(
            "Mine",
            latest(DashboardSectionSource {
                category_id: Some(mine.id),
                ..Default::default()
            }),
        )],
    )
    .await
    .expect("layout");
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(mine.id)
        .execute(&pool)
        .await
        .expect("purge folder");
    let view = db::get_dashboard(&pool, owner.id, id, &VaultKeyring::default())
        .await
        .expect("view");
    assert!(view.sections[0].items.is_empty());
}

#[tokio::test]
async fn items_carry_the_viewers_opens() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool, "counts").await;
    let a = bookmark(
        &pool,
        &owner,
        CreateBookmark {
            is_unread: true,
            is_favorite: true,
            ..at("https://counts.example/a")
        },
    )
    .await;
    let b = bookmark(&pool, &owner, at("https://counts.example/b")).await;
    bookmark(&pool, &owner, at("https://counts.example/never")).await;
    for _ in 0..3 {
        db::record_bookmark_open(&pool, owner.id, a)
            .await
            .expect("open a");
    }
    db::record_bookmark_open(&pool, owner.id, b)
        .await
        .expect("open b");
    // Two of a's opens fall outside the 30-day window.
    sqlx::query(
        "UPDATE bookmark_opens SET opened_at = now() - interval '40 days'
         WHERE ctid IN (SELECT ctid FROM bookmark_opens WHERE user_id = $1 AND bookmark_id = $2 LIMIT 2)",
    )
    .bind(owner.id)
    .bind(a)
    .execute(&pool)
    .await
    .expect("age");

    let opened = |window| DashboardSectionSettings {
        kind: DashboardSectionKind::MostOpened,
        open_window: window,
        ..Default::default()
    };
    let id = board(&pool, &owner).await;
    let view = put(
        &pool,
        &owner,
        id,
        vec![
            section("All time", opened(DashboardOpenWindow::AllTime)),
            section("30 days", opened(DashboardOpenWindow::Last30Days)),
            section("Latest", latest(DashboardSectionSource::default())),
        ],
    )
    .await
    .expect("layout");

    let counts = |i: usize| -> Vec<(String, Option<i64>)> {
        view.sections[i]
            .items
            .iter()
            .map(|it| (it.bookmark.url.clone(), it.open_count))
            .collect()
    };
    assert_eq!(
        counts(0),
        [
            ("https://counts.example/a".to_string(), Some(3)),
            ("https://counts.example/b".to_string(), Some(1)),
        ]
    );
    // A tie in the window goes to the one opened last.
    assert_eq!(
        counts(1),
        [
            ("https://counts.example/b".to_string(), Some(1)),
            ("https://counts.example/a".to_string(), Some(1)),
        ]
    );
    let never = view.sections[2]
        .items
        .iter()
        .find(|it| it.bookmark.url.ends_with("/never"))
        .expect("never");
    assert_eq!(never.open_count, None);
    assert!(never.last_opened_at.is_none());
}
