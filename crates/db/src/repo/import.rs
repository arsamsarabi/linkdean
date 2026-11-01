use linkdean_core::import_export::{leaf_name, path_key, ParsedImport};
use linkdean_core::VaultKeyring;
use linkdean_core::{BookmarkExport, CategoryExport, CreateCategory, Error, ImportJob, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use super::bookmarks::{create_bookmark, list_bookmarks};
use super::categories::{create_category, list_owned_categories};

pub async fn create_import_job(pool: &PgPool, user_id: Uuid, source: &str) -> Result<ImportJob> {
    sqlx::query_as::<_, ImportJob>(
        r#"
        INSERT INTO import_jobs (owner_user_id, source, status)
        VALUES ($1, $2, 'running')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(source)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn finish_import_job(
    pool: &PgPool,
    id: Uuid,
    total: i32,
    imported: i32,
    failed: i32,
    error: Option<&str>,
) -> Result<ImportJob> {
    let status = if error.is_some() { "failed" } else { "done" };
    sqlx::query_as::<_, ImportJob>(
        r#"
        UPDATE import_jobs
        SET status=$2, total=$3, imported=$4, failed=$5, error=$6, updated_at=now()
        WHERE id=$1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(total)
    .bind(imported)
    .bind(failed)
    .bind(error)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn apply_parsed_import(
    pool: &PgPool,
    user_id: Uuid,
    parsed: ParsedImport,
) -> Result<(i32, i32, i32)> {
    let mut path_to_id: HashMap<String, Uuid> = HashMap::new();
    for (cat, path_entry) in parsed.categories.into_iter().zip(parsed.category_paths) {
        let (path, parent_path) = path_key(&path_entry);
        let parent_id = parent_path.and_then(|p| path_to_id.get(p).copied());
        let created = create_category(
            pool,
            user_id,
            CreateCategory {
                name: leaf_name(path).to_string(),
                parent_id,
                is_hidden: cat.is_hidden,
                is_vault: false,
                icon: cat.icon,
            },
        )
        .await?;
        path_to_id.insert(path.to_string(), created.id);
    }

    let total = parsed.bookmarks.len() as i32;
    let mut imported = 0i32;
    let mut failed = 0i32;
    for (mut bm, cat_path) in parsed.bookmarks {
        bm.category_id = cat_path.and_then(|p| path_to_id.get(&p).copied());
        match create_bookmark(pool, user_id, bm, &VaultKeyring::default()).await {
            Ok(_) => imported += 1,
            Err(e) => {
                tracing::warn!(?e, "import bookmark failed");
                failed += 1;
            }
        }
    }
    Ok((total, imported, failed))
}

pub async fn export_data(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(Vec<CategoryExport>, Vec<BookmarkExport>)> {
    let cats = list_owned_categories(pool, user_id, true, false).await?;
    let cat_exports = cats
        .into_iter()
        .map(|c| CategoryExport {
            id: c.id,
            parent_id: c.parent_id,
            name: c.name,
            is_hidden: c.is_hidden,
            is_vault: c.is_vault,
            icon: c.icon,
            position: c.position,
        })
        .collect::<Vec<_>>();

    let bookmarks =
        list_bookmarks(pool, user_id, None, false, &VaultKeyring::default(), false).await?;
    let owned_ids: std::collections::HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM bookmarks WHERE owner_user_id = $1 AND is_vault = FALSE",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    let bm_exports = bookmarks
        .into_iter()
        .filter(|b| owned_ids.contains(&b.id))
        .map(|b| BookmarkExport {
            url: b.url,
            title: b.title,
            description: b.description,
            notes: b.notes,
            category_id: b.category_id,
            tags: b.tags,
            is_vault: false,
        })
        .collect();
    Ok((cat_exports, bm_exports))
}
