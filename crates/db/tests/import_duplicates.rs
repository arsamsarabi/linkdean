//! Import reuses folders by path, and lets the person decide about bookmarks already in
//! the library.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::import_export::{parse_netscape, DuplicateAction, DuplicateDecisions};
use linkdean_core::{CreateBookmark, CreateCategory, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping import duplicate tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool) -> User {
    let email = format!("dup-{}@import.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn folder(pool: &PgPool, owner: &User, name: &str, parent_id: Option<Uuid>) -> Uuid {
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
    .expect("folder")
    .id
}

async fn folder_count(pool: &PgPool, owner: &User) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM categories WHERE owner_user_id = $1 AND deleted_at IS NULL",
    )
    .bind(owner.id)
    .fetch_one(pool)
    .await
    .expect("count")
}

fn html(body: &str) -> String {
    format!("<DL><p>\n{body}</DL><p>\n")
}

fn a(url: &str, title: &str) -> String {
    format!("<DT><A HREF=\"{url}\">{title}</A>\n")
}

fn h3(name: &str, inner: &str) -> String {
    format!("<DT><H3>{name}</H3>\n<DL><p>\n{inner}</DL><p>\n")
}

#[tokio::test]
async fn importing_the_same_file_twice_reuses_its_folders() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let file = html(&h3("Work", &h3("Rust", &a("https://f1.example", "F1"))));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap())
        .await
        .unwrap();
    assert_eq!(folder_count(&pool, &me).await, 2);
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap())
        .await
        .unwrap();
    assert_eq!(folder_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn a_new_child_goes_under_a_reused_parent() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let work = folder(&pool, &me, "Work", None).await;
    let file = html(&h3("Work", &h3("Go", &a("https://f2.example", "F2"))));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap())
        .await
        .unwrap();
    let (parent,): (Option<Uuid>,) =
        sqlx::query_as("SELECT parent_id FROM categories WHERE owner_user_id = $1 AND name = 'Go'")
            .bind(me.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(parent, Some(work));
    assert_eq!(folder_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn same_name_siblings_in_a_file_share_one_folder() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let file = html(&format!(
        "{}{}",
        h3("Reading", &a("https://f3.example", "F3")),
        h3("Reading", &a("https://f4.example", "F4"))
    ));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap())
        .await
        .unwrap();
    assert_eq!(folder_count(&pool, &me).await, 1);
}

#[tokio::test]
async fn names_match_exactly_and_vault_folders_are_not_reused() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    folder(&pool, &me, "work", None).await;
    let file = html(&h3("Work", &a("https://f5.example", "F5")));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap())
        .await
        .unwrap();
    assert_eq!(folder_count(&pool, &me).await, 2);
}

async fn bookmark(pool: &PgPool, owner: &User, url: &str, title: &str, folders: Vec<Uuid>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: url.into(),
            title: Some(title.into()),
            category_ids: folders,
            tags: vec!["old".into()],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn view_of(pool: &PgPool, owner: &User, id: Uuid) -> linkdean_core::BookmarkView {
    let raw = db::get_bookmark_raw(pool, owner.id, id).await.expect("raw");
    db::bookmark_to_view(pool, raw, &VaultKeyring::default(), owner.id)
        .await
        .expect("view")
        .expect("visible")
}

async fn bookmark_count(pool: &PgPool, owner: &User) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM bookmarks WHERE owner_user_id = $1 AND deleted_at IS NULL",
    )
    .bind(owner.id)
    .fetch_one(pool)
    .await
    .expect("count")
}

#[tokio::test]
async fn preview_lists_exact_url_duplicates_and_writes_nothing() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let work = folder(&pool, &me, "Work", None).await;
    let url = format!("https://p.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Mine", vec![work]).await;
    let file = html(&format!(
        "{}{}{}",
        a("https://fresh.example", "Fresh"),
        h3("Work", &a(&url, "Theirs")),
        a(&format!("{url}/"), "Slash differs")
    ));
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&file).unwrap())
        .await
        .unwrap();

    assert_eq!((preview.total, preview.new), (3, 2));
    assert_eq!(preview.duplicates.len(), 1);
    let dup = &preview.duplicates[0];
    assert_eq!((dup.index, dup.url.as_str()), (1, url.as_str()));
    assert_eq!(dup.existing.id, id);
    assert_eq!(dup.existing.title, "Mine");
    assert_eq!(dup.existing.folders, vec!["Work".to_string()]);
    assert_eq!(dup.incoming.title.as_deref(), Some("Theirs"));
    assert_eq!(dup.incoming.folders, vec!["Work".to_string()]);
    assert_eq!((preview.folders.reused, preview.folders.created), (1, 0));
    assert_eq!(bookmark_count(&pool, &me).await, 1);
}

#[tokio::test]
async fn trashed_and_vault_bookmarks_are_not_duplicates() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let url = format!("https://t.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Gone", vec![]).await;
    sqlx::query("UPDATE bookmarks SET deleted_at = now() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let vault_url = format!("https://v.example/{}", Uuid::new_v4());
    let vid = bookmark(&pool, &me, &vault_url, "Vaulted", vec![]).await;
    let (vault, _, _) = db::create_vault(&pool, me.id, "Vault", "vault-passphrase")
        .await
        .expect("vault");
    // Only the filter is under test: real vault rows also hold their URL encrypted.
    sqlx::query("UPDATE bookmarks SET is_vault = TRUE, vault_id = $2 WHERE id = $1")
        .bind(vid)
        .bind(vault.id)
        .execute(&pool)
        .await
        .unwrap();
    let file = html(&format!("{}{}", a(&url, "T"), a(&vault_url, "V")));
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&file).unwrap())
        .await
        .unwrap();
    assert!(preview.duplicates.is_empty());
    assert_eq!(preview.new, 2);
}

#[tokio::test]
async fn several_copies_in_the_library_report_the_latest() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let url = format!("https://l.example/{}", Uuid::new_v4());
    let older = bookmark(&pool, &me, &url, "Older", vec![]).await;
    let newer = bookmark(&pool, &me, &url, "Newer", vec![]).await;
    sqlx::query("UPDATE bookmarks SET updated_at = now() - interval '1 day' WHERE id = $1")
        .bind(older)
        .execute(&pool)
        .await
        .unwrap();
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&html(&a(&url, "X"))).unwrap())
        .await
        .unwrap();
    assert_eq!(preview.duplicates[0].existing.id, newer);
}

async fn import_with(
    pool: &PgPool,
    me: &User,
    file: &str,
    decisions: DuplicateDecisions,
) -> db::ImportCounts {
    let parsed = parse_netscape(file).unwrap();
    let plan = db::plan_import(pool, me.id, &parsed, &decisions)
        .await
        .unwrap();
    let mut counts = db::ImportCounts::default();
    db::apply_parsed_import_counted(pool, me.id, parsed, &plan, &mut counts)
        .await
        .unwrap();
    counts
}

fn all(action: DuplicateAction) -> DuplicateDecisions {
    DuplicateDecisions {
        default: action,
        ..Default::default()
    }
}

#[tokio::test]
async fn skip_creates_nothing_for_the_duplicate() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let url = format!("https://s.example/{}", Uuid::new_v4());
    bookmark(&pool, &me, &url, "Mine", vec![]).await;
    let file = html(&format!(
        "{}{}",
        a(&url, "Theirs"),
        a("https://s-new.example", "New")
    ));
    let c = import_with(&pool, &me, &file, all(DuplicateAction::Skip)).await;
    assert_eq!((c.total, c.imported, c.skipped, c.replaced), (2, 1, 1, 0));
    assert_eq!(bookmark_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn keep_creates_a_second_copy_and_is_the_default() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let url = format!("https://k.example/{}", Uuid::new_v4());
    bookmark(&pool, &me, &url, "Mine", vec![]).await;
    let c = import_with(
        &pool,
        &me,
        &html(&a(&url, "Theirs")),
        DuplicateDecisions::default(),
    )
    .await;
    assert_eq!((c.imported, c.skipped, c.replaced), (1, 0, 0));
    assert_eq!(bookmark_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn replace_updates_in_place_and_keeps_the_id() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let old = folder(&pool, &me, "Old", None).await;
    let url = format!("https://r.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Mine", vec![old]).await;
    db::update_bookmark(
        &pool,
        me.id,
        id,
        linkdean_core::UpdateBookmark {
            is_favorite: Some(true),
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .unwrap();
    let file = html(&h3(
        "New",
        &format!("<DT><A HREF=\"{url}\" TAGS=\"fresh\">Theirs</A>\n<DD>Their notes\n"),
    ));
    let c = import_with(&pool, &me, &file, all(DuplicateAction::Replace)).await;
    assert_eq!((c.imported, c.replaced), (0, 1));
    assert_eq!(bookmark_count(&pool, &me).await, 1);

    let view = view_of(&pool, &me, id).await;
    assert_eq!(view.title, "Theirs");
    assert_eq!(view.description.as_deref(), Some("Their notes"));
    assert_eq!(view.tags, vec!["fresh".to_string()]);
    assert_eq!(
        view.categories
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>(),
        vec!["New"]
    );
    assert!(view.is_favorite);
}

#[tokio::test]
async fn replace_keeps_values_the_file_does_not_have() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let old = folder(&pool, &me, "Old", None).await;
    let url = format!("https://r2.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Mine", vec![old]).await;
    // No title, no tags, no folder in the file.
    let c = import_with(
        &pool,
        &me,
        &html(&format!("<DT><A HREF=\"{url}\"></A>\n")),
        all(DuplicateAction::Replace),
    )
    .await;
    assert_eq!(c.replaced, 1);
    let view = view_of(&pool, &me, id).await;
    assert_eq!(view.title, "Mine");
    assert_eq!(view.tags, vec!["old".to_string()]);
    assert_eq!(
        view.categories
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>(),
        vec!["Old"]
    );
}

#[tokio::test]
async fn an_override_beats_the_default_and_the_plan_counts_creates() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let u1 = format!("https://o1.example/{}", Uuid::new_v4());
    let u2 = format!("https://o2.example/{}", Uuid::new_v4());
    bookmark(&pool, &me, &u1, "One", vec![]).await;
    bookmark(&pool, &me, &u2, "Two", vec![]).await;
    let file = html(&format!(
        "{}{}{}",
        a(&u1, "1"),
        a(&u2, "2"),
        a("https://o3.example", "3")
    ));
    let parsed = parse_netscape(&file).unwrap();
    let mut decisions = all(DuplicateAction::Skip);
    decisions.overrides.insert(1, DuplicateAction::Keep);
    let plan = db::plan_import(&pool, me.id, &parsed, &decisions)
        .await
        .unwrap();
    assert_eq!(plan.creates(), 2);
}

#[tokio::test]
async fn a_bookmark_added_after_the_preview_gets_the_default() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let me = person(&pool).await;
    let url = format!("https://late.example/{}", Uuid::new_v4());
    let file = html(&a(&url, "Late"));
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&file).unwrap())
        .await
        .unwrap();
    assert!(preview.duplicates.is_empty());
    bookmark(&pool, &me, &url, "Added meanwhile", vec![]).await;
    let c = import_with(&pool, &me, &file, all(DuplicateAction::Skip)).await;
    assert_eq!((c.imported, c.skipped), (0, 1));
}
