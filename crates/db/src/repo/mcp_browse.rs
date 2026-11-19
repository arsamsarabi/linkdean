//! Empty-`q` MCP browse: folder tree (+ optional tag) without Meili or vault unlock.

use super::bookmarks::bookmarks_to_views;
use super::shares::require_category_read;
use linkdean_core::{Bookmark, BookmarkView, Error, Result, VaultKeyring};
use sqlx::PgPool;
use uuid::Uuid;

fn clamp_limit(limit: i64) -> i64 {
    if (1..=100).contains(&limit) {
        limit
    } else {
        50
    }
}

/// Read-checked folder id plus non-vault, non-deleted descendants (root included).
pub async fn mcp_folder_descendant_ids(
    pool: &PgPool,
    user_id: Uuid,
    folder_id: Uuid,
) -> Result<Vec<Uuid>> {
    let (cat, _) = require_category_read(pool, user_id, folder_id).await?;
    if cat.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    if cat.is_vault {
        return Err(Error::BadRequest(
            "vault folders can't be browsed via MCP".into(),
        ));
    }
    let ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        WITH RECURSIVE tree AS (
            SELECT id FROM categories WHERE id = $1
            UNION ALL
            SELECT c.id FROM categories c JOIN tree t ON c.parent_id = t.id
            WHERE c.deleted_at IS NULL AND c.is_vault = FALSE
        )
        SELECT id FROM tree
        "#,
    )
    .bind(folder_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(ids)
}

/// Library browse for MCP empty-`q`: optional folder (with descendants) and/or tag.
/// Tag scope is the caller's own bookmarks only (same as Agent Skills). Never unlocks vault.
pub async fn mcp_browse_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    folder: Option<Uuid>,
    tag: Option<&str>,
    limit: i64,
) -> Result<Vec<BookmarkView>> {
    let limit = clamp_limit(limit);
    let tag = tag.map(str::trim).filter(|t| !t.is_empty());
    if folder.is_none() && tag.is_none() {
        return Err(Error::BadRequest(
            "folder or tag is required for browse".into(),
        ));
    }

    let folder_ids = if let Some(id) = folder {
        Some(mcp_folder_descendant_ids(pool, user_id, id).await?)
    } else {
        None
    };

    let rows = match (folder_ids.as_ref(), tag) {
        (Some(ids), Some(tag)) => {
            sqlx::query_as::<_, Bookmark>(
                r#"
                SELECT * FROM (
                    SELECT DISTINCT ON (b.id) b.*
                    FROM bookmarks b
                    JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                    JOIN tags t ON t.id = bt.tag_id
                    WHERE b.owner_user_id = $1
                      AND t.owner_user_id = $1
                      AND lower(t.name) = lower($2)
                      AND b.deleted_at IS NULL AND b.is_vault = FALSE
                      AND (
                        b.category_id = ANY($3)
                        OR EXISTS (
                          SELECT 1 FROM bookmark_categories bc
                          WHERE bc.bookmark_id = b.id AND bc.category_id = ANY($3)
                        )
                      )
                    ORDER BY b.id
                ) s
                ORDER BY updated_at DESC, id
                LIMIT $4
                "#,
            )
            .bind(user_id)
            .bind(tag)
            .bind(ids)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (Some(ids), None) => {
            sqlx::query_as::<_, Bookmark>(
                r#"
                SELECT * FROM (
                    SELECT DISTINCT ON (b.id) b.*
                    FROM bookmarks b
                    WHERE b.deleted_at IS NULL AND b.is_vault = FALSE
                      AND (b.owner_user_id = $1 OR b.is_private = FALSE)
                      AND (
                        b.category_id = ANY($2)
                        OR EXISTS (
                          SELECT 1 FROM bookmark_categories bc
                          WHERE bc.bookmark_id = b.id AND bc.category_id = ANY($2)
                        )
                      )
                    ORDER BY b.id
                ) s
                ORDER BY updated_at DESC, id
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(ids)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (None, Some(tag)) => {
            sqlx::query_as::<_, Bookmark>(
                r#"
                SELECT * FROM (
                    SELECT DISTINCT ON (b.id) b.*
                    FROM bookmarks b
                    JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                    JOIN tags t ON t.id = bt.tag_id
                    WHERE b.owner_user_id = $1
                      AND t.owner_user_id = $1
                      AND lower(t.name) = lower($2)
                      AND b.deleted_at IS NULL AND b.is_vault = FALSE
                    ORDER BY b.id
                ) s
                ORDER BY updated_at DESC, id
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(tag)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (None, None) => unreachable!("checked above"),
    }
    .map_err(|e| Error::msg(e.to_string()))?;

    let none = VaultKeyring::default();
    bookmarks_to_views(pool, rows, user_id, |_| &none).await
}
