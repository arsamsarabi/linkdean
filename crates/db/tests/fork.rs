//! Forking a folder copies only live content and lands whole or not at all.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::{CreateBookmark, CreateCategory, ForkCategory, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping fork tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn user(pool: &PgPool) -> User {
    let email = format!("fork-{}@fork.test", Uuid::new_v4());
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

async fn bookmark(pool: &PgPool, owner: &User, url: &str, folder: Uuid) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: url.into(),
            title: Some(url.into()),
            category_id: Some(folder),
            category_ids: vec![folder],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn counts(pool: &PgPool, owner: &User) -> (i64, i64) {
    sqlx::query_as(
        r#"
        SELECT (SELECT COUNT(*) FROM categories WHERE owner_user_id = $1),
               (SELECT COUNT(*) FROM bookmarks WHERE owner_user_id = $1)
        "#,
    )
    .bind(owner.id)
    .fetch_one(pool)
    .await
    .expect("counts")
}

#[tokio::test]
async fn a_fork_leaves_trashed_folders_and_bookmarks_behind() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let root = folder(&pool, &owner, "Root", None).await;
    let kept = folder(&pool, &owner, "Kept", Some(root)).await;
    let binned = folder(&pool, &owner, "Binned", Some(root)).await;
    bookmark(&pool, &owner, "https://live.example/", kept).await;
    bookmark(&pool, &owner, "https://in-binned.example/", binned).await;
    let gone = bookmark(&pool, &owner, "https://gone.example/", root).await;
    db::delete_category(&pool, owner.id, binned)
        .await
        .expect("trash folder");
    db::delete_bookmark(&pool, owner.id, gone)
        .await
        .expect("trash bookmark");

    let (copy, copied) = db::fork_category(&pool, owner.id, root, ForkCategory { name: None })
        .await
        .expect("fork");
    let urls: Vec<String> = copied.into_iter().map(|b| b.url).collect();
    assert_eq!(urls, vec!["https://live.example/".to_string()]);
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM categories WHERE parent_id = $1 ORDER BY name")
            .bind(copy.id)
            .fetch_all(&pool)
            .await
            .expect("children");
    assert_eq!(names, vec!["Kept".to_string()]);
}

#[tokio::test]
async fn a_failed_fork_leaves_nothing_behind() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let owner = user(&pool).await;
    let root = folder(&pool, &owner, "Root", None).await;
    bookmark(&pool, &owner, "https://first.example/", root).await;
    // Copies of this one URL fail to insert, so the fork stops part way.
    let tag = Uuid::new_v4().simple().to_string();
    let bad_url = format!("https://bad-{tag}.example/");
    bookmark(&pool, &owner, &bad_url, root).await;
    sqlx::raw_sql(&format!(
        r#"
        CREATE FUNCTION fork_fail_{tag}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'copy refused'; END $$;
        CREATE TRIGGER fork_fail_{tag} BEFORE INSERT ON bookmarks
        FOR EACH ROW WHEN (NEW.url = '{bad_url}') EXECUTE FUNCTION fork_fail_{tag}();
        "#
    ))
    .execute(&pool)
    .await
    .expect("trigger");
    let before = counts(&pool, &owner).await;

    let result = db::fork_category(&pool, owner.id, root, ForkCategory { name: None }).await;
    sqlx::raw_sql(&format!(
        "DROP TRIGGER fork_fail_{tag} ON bookmarks; DROP FUNCTION fork_fail_{tag}();"
    ))
    .execute(&pool)
    .await
    .expect("drop trigger");
    assert!(result.is_err(), "fork should fail on the refused copy");
    assert_eq!(counts(&pool, &owner).await, before);
}
