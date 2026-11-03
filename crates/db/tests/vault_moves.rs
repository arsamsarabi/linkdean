//! Moving categories and bookmarks into, out of and between vaults.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{
    CreateBookmark, CreateCategory, CreateCategoryShare, CreateDashboard, Error,
    MoveToVaultRequest, ShareAccess, User, VaultKeyring, VaultMoveBlockerKind, VaultMoveOutcome,
};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping vault move tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

struct Setup {
    pool: PgPool,
    owner: User,
    vault: Uuid,
    keyring: VaultKeyring,
}

async fn setup() -> Option<Setup> {
    let pool = pool_or_skip().await?;
    let owner = person(&pool, "owner").await;
    let (vault, key, _) = db::create_vault(&pool, owner.id, "Dest", "dest-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(vault.id, key);
    Some(Setup {
        pool,
        owner,
        vault: vault.id,
        keyring,
    })
}

async fn person(pool: &PgPool, prefix: &str) -> User {
    let email = format!("{prefix}-{}@moves.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("account-password"), None, None, None)
        .await
        .expect("user")
}

async fn category(s: &Setup, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        &s.pool,
        s.owner.id,
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

async fn bookmark(s: &Setup, url: &str, category_ids: Vec<Uuid>, tags: Vec<String>) -> Uuid {
    db::create_bookmark(
        &s.pool,
        s.owner.id,
        CreateBookmark {
            url: url.into(),
            title: Some("Rust docs".into()),
            notes: Some("private note".into()),
            category_ids,
            tags,
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

fn request(categories: Vec<Uuid>, bookmarks: Vec<Uuid>, to: Option<Uuid>) -> MoveToVaultRequest {
    MoveToVaultRequest {
        category_ids: categories,
        bookmark_ids: bookmarks,
        destination_vault_id: to,
    }
}

async fn blockers(s: &Setup, req: MoveToVaultRequest) -> Vec<VaultMoveBlockerKind> {
    match db::move_to_vault(&s.pool, s.owner.id, req, &s.keyring)
        .await
        .expect("move")
    {
        VaultMoveOutcome::Blocked(found) => found.into_iter().map(|b| b.kind).collect(),
        VaultMoveOutcome::Moved { .. } => panic!("expected blockers"),
    }
}

#[tokio::test]
async fn moving_in_wipes_plaintext_and_moving_out_restores_it() {
    let Some(s) = setup().await else {
        return;
    };
    let id = bookmark(&s, "https://rust.example/", vec![], vec!["lang".into()]).await;
    sqlx::query(
        "UPDATE bookmarks SET favicon_url = '/api/x', preview_image_url = '/api/y', has_preview = TRUE, link_check_status = 'ok' WHERE id = $1",
    )
    .bind(id)
    .execute(&s.pool)
    .await
    .expect("seed");
    sqlx::query(
        "INSERT INTO activity_events (user_id, action, subject_type, subject_id, subject_title, summary) VALUES ($1, 'created', 'bookmark', $2, 'Rust docs', 'Added Rust docs')",
    )
    .bind(s.owner.id)
    .bind(id)
    .execute(&s.pool)
    .await
    .expect("activity");

    let outcome = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move in");
    let VaultMoveOutcome::Moved { left_search, .. } = outcome else {
        panic!("expected a move");
    };
    assert_eq!(left_search, vec![id]);

    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert!(raw.is_vault);
    assert_eq!(raw.vault_id, Some(s.vault));
    assert_eq!(raw.url, "");
    assert_eq!(raw.title, "");
    assert_eq!(raw.notes, None);
    assert_eq!(raw.favicon_url, None);
    assert_eq!(raw.preview_image_url, None);
    assert!(!raw.has_preview);

    let (tags,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM bookmark_tags WHERE bookmark_id = $1")
            .bind(id)
            .fetch_one(&s.pool)
            .await
            .expect("tags");
    assert_eq!(tags, 0);
    let (title, summary): (String, String) =
        sqlx::query_as("SELECT subject_title, summary FROM activity_events WHERE subject_id = $1")
            .bind(id)
            .fetch_one(&s.pool)
            .await
            .expect("activity");
    assert!(!title.contains("Rust") && !summary.contains("Rust"));

    let view = db::bookmark_to_view(&s.pool, raw, &s.keyring, s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.url, "https://rust.example/");
    assert_eq!(view.notes.as_deref(), Some("private note"));
    assert_eq!(view.tags, vec!["lang".to_string()]);

    let outcome = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], None),
        &s.keyring,
    )
    .await
    .expect("move out");
    let VaultMoveOutcome::Moved { joined_search, .. } = outcome else {
        panic!("expected a move");
    };
    assert_eq!(joined_search, vec![id]);
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert!(!raw.is_vault);
    assert_eq!(raw.url, "https://rust.example/");
    assert_eq!(raw.vault_ciphertext, None);
    let view = db::bookmark_to_view(&s.pool, raw, &VaultKeyring::default(), s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.tags, vec!["lang".to_string()]);
}

#[tokio::test]
async fn moving_a_category_takes_its_subcategories_and_bookmarks() {
    let Some(s) = setup().await else {
        return;
    };
    let outer = category(&s, "Outer", None).await;
    let parent = category(&s, "Parent", Some(outer)).await;
    let child = category(&s, "Child", Some(parent)).await;
    let id = bookmark(&s, "https://nested.example/", vec![child], vec![]).await;

    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![parent], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move");

    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert_eq!(raw.vault_id, Some(s.vault));
    let cats = db::list_categories(&s.pool, s.owner.id, false, &s.keyring.vault_ids(), false)
        .await
        .expect("cats");
    let moved = cats.iter().find(|c| c.id == parent).expect("parent");
    assert_eq!(moved.parent_id, None);
    assert!(moved.is_vault);
    assert!(cats.iter().find(|c| c.id == child).expect("child").is_vault);
    assert!(!cats.iter().find(|c| c.id == outer).expect("outer").is_vault);
}

#[tokio::test]
async fn every_blocker_is_listed_and_nothing_moves() {
    let Some(s) = setup().await else {
        return;
    };
    let friend = person(&s.pool, "friend").await;
    let reading = category(&s, "Reading", None).await;
    let work = category(&s, "Work", None).await;
    let id = bookmark(&s, "https://both.example/", vec![reading, work], vec![]).await;

    db::create_share(
        &s.pool,
        s.owner.id,
        reading,
        CreateCategoryShare {
            email: friend.email.clone(),
            access: ShareAccess::Read,
        },
    )
    .await
    .expect("invite");
    db::enable_category_public_link(&s.pool, s.owner.id, reading)
        .await
        .expect("public link");
    let board = db::create_dashboard(
        &s.pool,
        s.owner.id,
        CreateDashboard {
            title: Some("Home".into()),
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("dashboard");
    let (section,): (Uuid,) = sqlx::query_as(
        "INSERT INTO dashboard_sections (dashboard_id, title) VALUES ($1, 'Top') RETURNING id",
    )
    .bind(board.id)
    .fetch_one(&s.pool)
    .await
    .expect("section");
    sqlx::query("INSERT INTO dashboard_items (section_id, bookmark_id) VALUES ($1, $2)")
        .bind(section)
        .bind(id)
        .execute(&s.pool)
        .await
        .expect("pin");

    let mut found = blockers(&s, request(vec![reading], vec![], Some(s.vault))).await;
    found.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(
        found,
        vec![
            VaultMoveBlockerKind::Dashboard,
            VaultMoveBlockerKind::OtherCategory,
            VaultMoveBlockerKind::PublicLink,
            VaultMoveBlockerKind::Shared,
        ]
    );

    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert!(!raw.is_vault);
    assert_eq!(raw.url, "https://both.example/");
}

#[tokio::test]
async fn moving_both_categories_together_is_not_blocked() {
    let Some(s) = setup().await else {
        return;
    };
    let first = category(&s, "First", None).await;
    let second = category(&s, "Second", None).await;
    let id = bookmark(&s, "https://both.example/", vec![first, second], vec![]).await;

    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![first, second], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move");
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert_eq!(raw.vault_id, Some(s.vault));
}

#[tokio::test]
async fn moving_between_vaults_reencrypts_and_needs_both_unlocked() {
    let Some(mut s) = setup().await else {
        return;
    };
    let (other, other_key, _) = db::create_vault(&s.pool, s.owner.id, "Other", "other-passphrase")
        .await
        .expect("other");
    let id = bookmark(&s, "https://move.example/", vec![], vec![]).await;
    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move in");

    let locked = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(other.id)),
        &s.keyring,
    )
    .await;
    assert!(matches!(locked, Err(Error::VaultLocked)));

    s.keyring.insert(other.id, other_key);
    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![], vec![id], Some(other.id)),
        &s.keyring,
    )
    .await
    .expect("move across");

    let mut only_other = VaultKeyring::default();
    only_other.insert(other.id, other_key);
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    let view = db::bookmark_to_view(&s.pool, raw, &only_other, s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.url, "https://move.example/");
}

#[tokio::test]
async fn another_users_items_cannot_move() {
    let Some(s) = setup().await else {
        return;
    };
    let id = bookmark(&s, "https://mine.example/", vec![], vec![]).await;
    let stranger = person(&s.pool, "stranger").await;
    let (theirs, key, _) = db::create_vault(&s.pool, stranger.id, "Theirs", "their-passphrase")
        .await
        .expect("vault");
    let mut keyring = VaultKeyring::default();
    keyring.insert(theirs.id, key);

    let got = db::move_to_vault(
        &s.pool,
        stranger.id,
        request(vec![], vec![id], Some(theirs.id)),
        &keyring,
    )
    .await;
    assert!(matches!(got, Err(Error::NotFound)));
}

#[tokio::test]
async fn a_bookmark_cannot_leave_while_its_vault_category_stays() {
    let Some(s) = setup().await else {
        return;
    };
    let cat = category(&s, "Stays", None).await;
    let id = bookmark(&s, "https://stays.example/", vec![cat], vec![]).await;
    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![cat], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move in");

    let found = blockers(&s, request(vec![], vec![id], None)).await;
    assert_eq!(found, vec![VaultMoveBlockerKind::OtherCategory]);

    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![cat], vec![], None),
        &s.keyring,
    )
    .await
    .expect("move the category out");
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert!(!raw.is_vault);
}

#[tokio::test]
async fn moving_in_removes_tags_social_rows_audit_and_list_activity() {
    let Some(s) = setup().await else {
        return;
    };
    let cat = category(&s, "Secret plans", None).await;
    let only_here = format!("only-{}", Uuid::new_v4());
    let shared_tag = format!("shared-{}", Uuid::new_v4());
    let id = bookmark(
        &s,
        "https://plans.example/",
        vec![cat],
        vec![only_here.clone(), shared_tag.clone()],
    )
    .await;
    bookmark(
        &s,
        "https://other.example/",
        vec![],
        vec![shared_tag.clone()],
    )
    .await;
    let commenter = person(&s.pool, "commenter").await;
    for (kind, subject) in [("bookmark", id), ("category", cat)] {
        sqlx::query(
            "INSERT INTO social_comments (user_id, subject_type, subject_id, body) VALUES ($1, $2, $3, 'plaintext')",
        )
        .bind(commenter.id)
        .bind(kind)
        .bind(subject)
        .execute(&s.pool)
        .await
        .expect("comment");
    }
    sqlx::query(
        "INSERT INTO moderation_actions (user_id, action, subject_type, subject_id, subject_title) VALUES ($1, 'bookmark_private', 'bookmark', $2, 'Rust docs')",
    )
    .bind(s.owner.id)
    .bind(id)
    .execute(&s.pool)
    .await
    .expect("audit");
    sqlx::query(
        "INSERT INTO activity_events (user_id, action, subject_type, subject_id, subject_title, summary) VALUES ($1, 'created', 'category', $2, 'Secret plans', 'Added Secret plans')",
    )
    .bind(s.owner.id)
    .bind(cat)
    .execute(&s.pool)
    .await
    .expect("activity");

    let outcome = db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![cat], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move");
    assert!(matches!(outcome, VaultMoveOutcome::Moved { .. }));

    let tags: Vec<String> =
        sqlx::query_scalar("SELECT name FROM tags WHERE owner_user_id = $1 ORDER BY name")
            .bind(s.owner.id)
            .fetch_all(&s.pool)
            .await
            .expect("tags");
    assert_eq!(tags, vec![shared_tag], "only the tag still in use stays");

    let comments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM social_comments WHERE subject_id = $1 OR subject_id = $2",
    )
    .bind(id)
    .bind(cat)
    .fetch_one(&s.pool)
    .await
    .expect("comments");
    assert_eq!(comments, 0);

    let audit: String =
        sqlx::query_scalar("SELECT subject_title FROM moderation_actions WHERE subject_id = $1")
            .bind(id)
            .fetch_one(&s.pool)
            .await
            .expect("audit");
    assert_eq!(audit, "Vault item");

    let activity: Vec<String> = sqlx::query_scalar(
        "SELECT subject_title || ' ' || summary FROM activity_events WHERE subject_id = $1",
    )
    .bind(cat)
    .fetch_all(&s.pool)
    .await
    .expect("activity");
    assert!(!activity.is_empty());
    assert!(activity.iter().all(|a| !a.contains("Secret plans")));
}

#[tokio::test]
async fn a_profile_listed_folder_turns_private_in_a_vault() {
    let Some(s) = setup().await else {
        return;
    };
    let shown = category(&s, "Shown", None).await;
    let child = category(&s, "Shown child", Some(shown)).await;
    sqlx::query("UPDATE categories SET visibility = 'followers' WHERE id = $1")
        .bind(shown)
        .execute(&s.pool)
        .await
        .expect("followers");
    sqlx::query("UPDATE categories SET visibility = 'members' WHERE id = $1")
        .bind(child)
        .execute(&s.pool)
        .await
        .expect("members");

    db::move_to_vault(
        &s.pool,
        s.owner.id,
        request(vec![shown], vec![], Some(s.vault)),
        &s.keyring,
    )
    .await
    .expect("move");

    let visibility: Vec<String> =
        sqlx::query_scalar("SELECT visibility FROM categories WHERE id = ANY($1) AND is_vault")
            .bind(vec![shown, child])
            .fetch_all(&s.pool)
            .await
            .expect("visibility");
    assert_eq!(
        visibility,
        vec!["private".to_string(), "private".to_string()]
    );
}

#[tokio::test]
async fn an_edit_saved_during_a_move_is_moved_not_lost() {
    let Some(s) = setup().await else {
        return;
    };
    let id = bookmark(&s, "https://edit-race.example/", vec![], vec![]).await;

    // An edit holds the row lock, as `update_bookmark` does, and is not committed yet, so the
    // move reads the old content and then waits on the lock.
    let mut edit = s.pool.begin().await.expect("begin");
    sqlx::query("SELECT 1 FROM bookmarks WHERE id = $1 FOR UPDATE")
        .bind(id)
        .execute(&mut *edit)
        .await
        .expect("lock");
    sqlx::query("UPDATE bookmarks SET title = 'Edited mid-move', updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *edit)
        .await
        .expect("edit");

    let (pool, owner, keyring) = (s.pool.clone(), s.owner.id, s.keyring.clone());
    let req = request(vec![], vec![id], Some(s.vault));
    let mover = tokio::spawn(async move { db::move_to_vault(&pool, owner, req, &keyring).await });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    edit.commit().await.expect("commit edit");

    let outcome = mover.await.expect("join").expect("move");
    assert!(matches!(outcome, VaultMoveOutcome::Moved { .. }));
    let raw = db::get_bookmark_raw(&s.pool, s.owner.id, id)
        .await
        .expect("raw");
    assert!(raw.is_vault);
    assert_eq!(raw.title, "", "no plaintext left in the row");
    let view = db::bookmark_to_view(&s.pool, raw, &s.keyring, s.owner.id)
        .await
        .expect("view")
        .expect("visible");
    assert_eq!(view.title, "Edited mid-move");
}
