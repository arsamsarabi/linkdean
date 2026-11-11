//! Import reuses folders by path, and lets the person decide about bookmarks already in
//! the library.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::import_export::parse_netscape;
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
