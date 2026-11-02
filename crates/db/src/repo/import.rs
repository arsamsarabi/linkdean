use linkdean_core::import_export::{leaf_name, path_key, ParsedImport};
use linkdean_core::VaultKeyring;
use linkdean_core::{
    BookmarkExport, CategoryExport, CreateCategory, CreateDashboard, DashboardExport,
    DashboardSectionExport, Error, ImportJob, PutDashboardLayout, PutDashboardSection, Result,
};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::bookmarks::{create_bookmark, list_bookmarks};
use super::categories::{create_category, list_owned_categories};
use super::dashboards::{create_dashboard, put_dashboard};

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
                vault_id: None,
                icon: cat.icon,
            },
        )
        .await?;
        path_to_id.insert(path.to_string(), created.id);
    }

    let total = parsed.bookmarks.len() as i32;
    let mut imported = 0i32;
    let mut failed = 0i32;
    let mut new_ids: HashMap<Uuid, Uuid> = HashMap::new();
    for item in parsed.bookmarks {
        let mut bm = item.bookmark;
        bm.category_id = None;
        bm.category_ids = item
            .category_paths
            .iter()
            .filter_map(|p| path_to_id.get(p).copied())
            .collect();
        let created = match create_bookmark(pool, user_id, bm, &VaultKeyring::default()).await {
            Ok(view) => view,
            Err(e) => {
                tracing::warn!(?e, "import bookmark failed");
                failed += 1;
                continue;
            }
        };
        imported += 1;
        if let Some(source) = item.source_id {
            new_ids.insert(source, created.id);
        }
        if item.is_private || item.created_at.is_some() || item.updated_at.is_some() {
            sqlx::query(
                r#"
                UPDATE bookmarks
                SET is_private = $2,
                    created_at = COALESCE($3, created_at),
                    updated_at = COALESCE($4, updated_at)
                WHERE id = $1
                "#,
            )
            .bind(created.id)
            .bind(item.is_private)
            .bind(item.created_at)
            .bind(item.updated_at)
            .execute(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        }
    }

    // Imported dashboards are added beside the existing ones, never made the default.
    for dash in parsed.dashboards {
        let board = create_dashboard(
            pool,
            user_id,
            CreateDashboard {
                title: Some(dash.title),
            },
            &VaultKeyring::default(),
        )
        .await?;
        let sections = dash
            .sections
            .into_iter()
            .map(|sec| PutDashboardSection {
                title: sec.title,
                bookmark_ids: sec
                    .bookmark_ids
                    .iter()
                    .filter_map(|id| new_ids.get(id).copied())
                    .collect(),
            })
            .collect();
        put_dashboard(
            pool,
            user_id,
            board.id,
            PutDashboardLayout {
                title: None,
                sections,
            },
            &VaultKeyring::default(),
        )
        .await?;
    }
    Ok((total, imported, failed))
}

pub async fn export_data(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(
    Vec<CategoryExport>,
    Vec<BookmarkExport>,
    Vec<DashboardExport>,
)> {
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
    let owned_ids: HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM bookmarks WHERE owner_user_id = $1 AND is_vault = FALSE",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    let bm_exports: Vec<BookmarkExport> = bookmarks
        .into_iter()
        .filter(|b| owned_ids.contains(&b.id))
        .map(|b| {
            let category_ids: Vec<Uuid> = b.categories.iter().map(|c| c.id).collect();
            BookmarkExport {
                id: Some(b.id),
                url: b.url,
                title: b.title,
                description: b.description,
                notes: b.notes,
                category_id: category_ids.first().copied().or(b.category_id),
                category_ids,
                tags: b.tags,
                is_vault: false,
                is_favorite: b.is_favorite,
                is_unread: b.is_unread,
                is_pinned: b.is_pinned,
                is_private: b.is_private,
                created_at: Some(b.created_at),
                updated_at: Some(b.updated_at),
            }
        })
        .collect();

    let exported: HashSet<Uuid> = bm_exports.iter().filter_map(|b| b.id).collect();
    let dashboards = export_dashboards(pool, user_id, &exported).await?;
    Ok((cat_exports, bm_exports, dashboards))
}

/// The user's own live dashboards. Pins of bookmarks not in the export are left out.
async fn export_dashboards(
    pool: &PgPool,
    user_id: Uuid,
    exported: &HashSet<Uuid>,
) -> Result<Vec<DashboardExport>> {
    let boards: Vec<(Uuid, String, bool)> = sqlx::query_as(
        r#"
        SELECT id, title, is_default FROM dashboards
        WHERE owner_user_id = $1 AND deleted_at IS NULL
        ORDER BY created_at
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(boards.len());
    for (id, title, is_default) in boards {
        let rows: Vec<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
            r#"
            SELECT s.id, s.title, i.bookmark_id
            FROM dashboard_sections s
            LEFT JOIN dashboard_items i ON i.section_id = s.id
            WHERE s.dashboard_id = $1
            ORDER BY s.position, s.created_at, i.position, i.created_at
            "#,
        )
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        let mut sections: Vec<(Uuid, DashboardSectionExport)> = Vec::new();
        for (section_id, section_title, bookmark_id) in rows {
            if sections.last().is_none_or(|(sid, _)| *sid != section_id) {
                sections.push((
                    section_id,
                    DashboardSectionExport {
                        title: section_title,
                        bookmark_ids: Vec::new(),
                    },
                ));
            }
            if let (Some(bid), Some((_, sec))) = (bookmark_id, sections.last_mut()) {
                if exported.contains(&bid) {
                    sec.bookmark_ids.push(bid);
                }
            }
        }
        out.push(DashboardExport {
            title,
            is_default,
            sections: sections.into_iter().map(|(_, s)| s).collect(),
        });
    }
    Ok(out)
}
