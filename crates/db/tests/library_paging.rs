//! The library list: filters, pin-first sort and keyset paging, all in SQL.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, ShareAccess, User, VaultKeyring,
};
use linkdean_db::{self as db, BookmarkPageFilter, BookmarkSort};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping library paging tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool) -> User {
    let email = format!("page-{}@paging.test", Uuid::new_v4());
    let user = db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user");
    // Shares attach only to verified addresses.
    sqlx::query("UPDATE users SET email_verified_at = now() WHERE id = $1")
        .bind(user.id)
        .execute(pool)
        .await
        .expect("verify");
    user
}

async fn folder(pool: &PgPool, owner: &User, name: &str) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: name.into(),
            parent_id: None,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder")
    .id
}

async fn share(pool: &PgPool, owner: &User, folder: Uuid, invitee: &User) {
    db::create_share(
        pool,
        owner.id,
        folder,
        CreateCategoryShare {
            email: invitee.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("share");
    for invite in db::list_share_invites(pool, invitee.id)
        .await
        .expect("inbox")
    {
        db::accept_share_invite(pool, invitee.id, invite.id)
            .await
            .expect("accept");
    }
}

struct Seed<'a> {
    url: &'a str,
    title: &'a str,
    folders: Vec<Uuid>,
    tags: &'a [&'a str],
    pinned: bool,
    favorite: bool,
    unread: bool,
    /// Minutes after a fixed base time, for created_at and updated_at.
    minute: i64,
}

impl Default for Seed<'_> {
    fn default() -> Self {
        Seed {
            url: "https://example.com/",
            title: "",
            folders: vec![],
            tags: &[],
            pinned: false,
            favorite: false,
            unread: false,
            minute: 0,
        }
    }
}

async fn seed(pool: &PgPool, owner: &User, s: Seed<'_>) -> Uuid {
    let b = db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: s.url.into(),
            title: Some(s.title.into()),
            category_ids: s.folders,
            tags: s.tags.iter().map(|t| t.to_string()).collect(),
            is_pinned: s.pinned,
            is_favorite: s.favorite,
            is_unread: s.unread,
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark");
    sqlx::query(
        "UPDATE bookmarks SET created_at = timestamptz '2026-01-01 00:00:00+00' + make_interval(mins => $2::int),
                              updated_at = timestamptz '2026-01-01 00:00:00+00' + make_interval(mins => $2::int)
         WHERE id = $1",
    )
    .bind(b.id)
    .bind(s.minute as i32)
    .execute(pool)
    .await
    .expect("times");
    b.id
}

fn filter(sort: BookmarkSort, limit: i64) -> BookmarkPageFilter {
    BookmarkPageFilter {
        sort,
        limit,
        ..Default::default()
    }
}

/// Every page from the first to the last, as ids, plus the total of the first page.
async fn walk(pool: &PgPool, user: &User, f: BookmarkPageFilter) -> (Vec<Uuid>, i64) {
    let keys = VaultKeyring::default();
    let mut out = Vec::new();
    let mut cursor = None;
    let mut total = None;
    for _ in 0..100 {
        let page = db::list_bookmarks_page(
            pool,
            user.id,
            BookmarkPageFilter {
                cursor: cursor.clone(),
                ..f.clone()
            },
            &keys,
        )
        .await
        .expect("page");
        total.get_or_insert(page.total);
        out.extend(page.items.iter().map(|b| b.id));
        match page.next_cursor {
            Some(c) => cursor = Some(c),
            None => return (out, total.unwrap()),
        }
    }
    panic!("paging never ended");
}

#[tokio::test]
async fn pages_cover_every_match_once_pinned_first_with_ties_broken_by_id() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    // Three bookmarks share minute 5 and two share the title "same", so paging must break
    // ties by id in the same direction as the sort or it skips or repeats rows.
    let a = seed(
        &pool,
        &me,
        Seed {
            title: "Alpha",
            minute: 1,
            ..Default::default()
        },
    )
    .await;
    let b = seed(
        &pool,
        &me,
        Seed {
            title: "same",
            minute: 5,
            ..Default::default()
        },
    )
    .await;
    let c = seed(
        &pool,
        &me,
        Seed {
            title: "same",
            minute: 5,
            ..Default::default()
        },
    )
    .await;
    let d = seed(
        &pool,
        &me,
        Seed {
            title: "Delta",
            minute: 5,
            ..Default::default()
        },
    )
    .await;
    let e = seed(
        &pool,
        &me,
        Seed {
            title: "echo",
            minute: 9,
            pinned: true,
            ..Default::default()
        },
    )
    .await;
    let f = seed(
        &pool,
        &me,
        Seed {
            url: "https://zz.example/",
            title: "",
            minute: 3,
            ..Default::default()
        },
    )
    .await;

    let by_id = |mut v: Vec<Uuid>, desc: bool| {
        v.sort();
        if desc {
            v.reverse();
        }
        v
    };
    let tie_desc = by_id(vec![b, c, d], true);
    let tie_asc = by_id(vec![b, c, d], false);
    let same_asc = by_id(vec![b, c], false);
    let same_desc = by_id(vec![b, c], true);

    let mut updated_desc = vec![e];
    updated_desc.extend(&tie_desc);
    updated_desc.extend([f, a]);

    let mut updated_asc = vec![e, a, f];
    updated_asc.extend(&tie_asc);

    // Titles sort case-insensitively, and an empty title sorts by its whole URL, so
    // "https://zz.example/" lands between "delta" and "same".
    let mut title_asc = vec![e, a, d, f];
    title_asc.extend(&same_asc);

    let mut title_desc = vec![e];
    title_desc.extend(&same_desc);
    title_desc.extend([f, d, a]);

    for (sort, want) in [
        (BookmarkSort::UpdatedDesc, updated_desc.clone()),
        (BookmarkSort::CreatedDesc, updated_desc),
        (BookmarkSort::UpdatedAsc, updated_asc),
        (BookmarkSort::TitleAsc, title_asc),
        (BookmarkSort::TitleDesc, title_desc),
    ] {
        for limit in [1, 2, 4, 50] {
            let (got, total) = walk(&pool, &me, filter(sort, limit)).await;
            assert_eq!(got, want, "{sort:?} limit {limit}");
            assert_eq!(total, 6, "{sort:?} total");
        }
    }
}

#[tokio::test]
async fn filters_narrow_the_set_and_the_total() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let rust = folder(&pool, &me, "Rust").await;
    let both = seed(
        &pool,
        &me,
        Seed {
            folders: vec![rust],
            tags: &["rust", "async"],
            favorite: true,
            unread: true,
            minute: 10,
            ..Default::default()
        },
    )
    .await;
    let one_tag = seed(
        &pool,
        &me,
        Seed {
            folders: vec![rust],
            tags: &["rust"],
            favorite: true,
            minute: 20,
            ..Default::default()
        },
    )
    .await;
    let loose = seed(
        &pool,
        &me,
        Seed {
            tags: &["async"],
            unread: true,
            minute: 30,
            ..Default::default()
        },
    )
    .await;
    let trashed = seed(
        &pool,
        &me,
        Seed {
            minute: 40,
            ..Default::default()
        },
    )
    .await;
    db::delete_bookmark(&pool, me.id, trashed)
        .await
        .expect("trash");

    let base = filter(BookmarkSort::CreatedDesc, 1);
    let run = |f: BookmarkPageFilter| {
        let pool = pool.clone();
        let me = me.clone();
        async move { walk(&pool, &me, f).await }
    };

    assert_eq!(run(base.clone()).await, (vec![loose, one_tag, both], 3));
    // Every tag must be present.
    let tags = BookmarkPageFilter {
        tags: vec!["rust".into(), "async".into()],
        ..base.clone()
    };
    assert_eq!(run(tags).await, (vec![both], 1));
    // Every flag must be set.
    let flags = BookmarkPageFilter {
        flags: vec!["favorite".into(), "unread".into()],
        ..base.clone()
    };
    assert_eq!(run(flags).await, (vec![both], 1));
    let folder_only = BookmarkPageFilter {
        folder: Some(rust.to_string()),
        ..base.clone()
    };
    assert_eq!(run(folder_only).await, (vec![one_tag, both], 2));
    let unsorted = BookmarkPageFilter {
        folder: Some("uncategorized".into()),
        ..base.clone()
    };
    assert_eq!(run(unsorted).await, (vec![loose], 1));
    let since = BookmarkPageFilter {
        since: Some("2026-01-01T00:15:00Z".parse().unwrap()),
        ..base.clone()
    };
    assert_eq!(run(since).await, (vec![loose, one_tag], 2));
    // Deleted items only when the person shows them in the library.
    let with_trash = BookmarkPageFilter {
        include_deleted: true,
        ..base.clone()
    };
    assert_eq!(
        run(with_trash).await,
        (vec![trashed, loose, one_tag, both], 4)
    );

    // The id list for select-all matches the pages.
    let ids = db::list_bookmark_ids_matching(&pool, me.id, base, &VaultKeyring::default())
        .await
        .expect("ids");
    assert_eq!(ids, vec![loose, one_tag, both]);
}

#[tokio::test]
async fn the_whole_library_includes_folders_shared_with_you_once() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let friend = person(&pool).await;
    let theirs = folder(&pool, &friend, "Shared reading").await;
    let other = folder(&pool, &friend, "Not shared").await;
    share(&pool, &friend, theirs, &me).await;

    let mine = seed(
        &pool,
        &me,
        Seed {
            minute: 1,
            ..Default::default()
        },
    )
    .await;
    let shared = seed(
        &pool,
        &friend,
        Seed {
            folders: vec![theirs, other],
            minute: 2,
            ..Default::default()
        },
    )
    .await;
    let _private = seed(
        &pool,
        &friend,
        Seed {
            folders: vec![other],
            minute: 3,
            ..Default::default()
        },
    )
    .await;
    let shared_trashed = seed(
        &pool,
        &friend,
        Seed {
            folders: vec![theirs],
            minute: 4,
            ..Default::default()
        },
    )
    .await;
    db::delete_bookmark(&pool, friend.id, shared_trashed)
        .await
        .expect("trash");

    let base = filter(BookmarkSort::CreatedDesc, 1);
    assert_eq!(
        walk(&pool, &me, base.clone()).await,
        (vec![shared, mine], 2)
    );
    let in_shared = BookmarkPageFilter {
        folder: Some(theirs.to_string()),
        ..base.clone()
    };
    assert_eq!(walk(&pool, &me, in_shared).await, (vec![shared], 1));
    // A folder that was not shared stays closed.
    let closed = BookmarkPageFilter {
        folder: Some(other.to_string()),
        ..base
    };
    assert!(
        db::list_bookmarks_page(&pool, me.id, closed, &VaultKeyring::default())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_cursor_from_another_sort_is_refused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    seed(
        &pool,
        &me,
        Seed {
            minute: 1,
            ..Default::default()
        },
    )
    .await;
    seed(
        &pool,
        &me,
        Seed {
            minute: 2,
            ..Default::default()
        },
    )
    .await;
    let keys = VaultKeyring::default();
    let first = db::list_bookmarks_page(&pool, me.id, filter(BookmarkSort::UpdatedDesc, 1), &keys)
        .await
        .expect("page");
    let cursor = first.next_cursor.expect("more");
    let wrong = BookmarkPageFilter {
        cursor: Some(cursor),
        ..filter(BookmarkSort::TitleAsc, 1)
    };
    assert!(db::list_bookmarks_page(&pool, me.id, wrong, &keys)
        .await
        .is_err());
    let junk = BookmarkPageFilter {
        cursor: Some("not-a-cursor".into()),
        ..filter(BookmarkSort::UpdatedDesc, 1)
    };
    assert!(db::list_bookmarks_page(&pool, me.id, junk, &keys)
        .await
        .is_err());
}
