//! Bookmarks for an Agent Skill made from a folder (with its subfolders) or a tag.

use super::shares::require_category_read;
use linkdean_core::agent_skill::{SkillLink, SkillSource, SkillSourceKind, MAX_SKILL_BOOKMARKS};
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// What a skill is made from.
#[derive(Debug, Clone)]
pub enum SkillFrom {
    Folder(Uuid),
    Tag(String),
}

/// The links for a skill, the source label, and how many matching bookmarks were left out.
pub struct SkillBookmarks {
    pub source: SkillSource,
    pub links: Vec<SkillLink>,
    pub truncated: usize,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    owner_user_id: Uuid,
    url: String,
    title: String,
    description: Option<String>,
    notes: Option<String>,
    path: Vec<String>,
}

/// Reads the bookmarks `user_id` can see in the source. A folder needs read access (owned or
/// shared); vault folders and vault bookmarks are never included, nor deleted ones, nor someone
/// else's "Only me" bookmarks or notes. A tag covers the user's own bookmarks only.
pub async fn agent_skill_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    from: &SkillFrom,
) -> Result<SkillBookmarks> {
    let limit = (MAX_SKILL_BOOKMARKS + 1) as i64;
    let (source, rows) = match from {
        SkillFrom::Folder(id) => {
            let (cat, _) = require_category_read(pool, user_id, *id).await?;
            if cat.deleted_at.is_some() {
                return Err(Error::NotFound);
            }
            if cat.is_vault {
                return Err(Error::BadRequest(
                    "vault folders can't be made into a skill".into(),
                ));
            }
            let rows = sqlx::query_as::<_, Row>(
                r#"
                WITH RECURSIVE tree AS (
                    SELECT id, ARRAY[]::text[] AS path FROM categories WHERE id = $1
                    UNION ALL
                    SELECT c.id, t.path || c.name
                    FROM categories c JOIN tree t ON c.parent_id = t.id
                    WHERE c.deleted_at IS NULL AND c.is_vault = FALSE
                ),
                hits AS (
                    SELECT DISTINCT ON (b.id) b.id, b.owner_user_id, b.url, b.title,
                        b.description, b.notes, t.path
                    FROM tree t
                    JOIN bookmarks b ON b.category_id = t.id OR EXISTS (
                        SELECT 1 FROM bookmark_categories bc
                        WHERE bc.bookmark_id = b.id AND bc.category_id = t.id)
                    WHERE b.deleted_at IS NULL AND b.is_vault = FALSE
                      AND (b.owner_user_id = $2 OR b.is_private = FALSE)
                    ORDER BY b.id, cardinality(t.path)
                )
                SELECT * FROM hits
                ORDER BY path, lower(title), id
                LIMIT $3
                "#,
            )
            .bind(id)
            .bind(user_id)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            (
                SkillSource {
                    kind: SkillSourceKind::Folder,
                    label: cat.name,
                },
                rows,
            )
        }
        SkillFrom::Tag(tag) => {
            let tag = tag.trim();
            if tag.is_empty() {
                return Err(Error::BadRequest("tag is required".into()));
            }
            // The tag as the user wrote it, not as the request spelled it.
            let label: String = sqlx::query_scalar(
                "SELECT name FROM tags WHERE owner_user_id = $1 AND lower(name) = lower($2) \
                 ORDER BY name COLLATE \"C\" LIMIT 1",
            )
            .bind(user_id)
            .bind(tag)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?
            .ok_or(Error::NotFound)?;
            let rows = sqlx::query_as::<_, Row>(
                r#"
                SELECT DISTINCT b.id, b.owner_user_id, b.url, b.title, b.description, b.notes,
                    ARRAY[]::text[] AS path, lower(b.title) AS sort_title
                FROM bookmarks b
                JOIN bookmark_tags bt ON bt.bookmark_id = b.id
                JOIN tags t ON t.id = bt.tag_id
                WHERE b.owner_user_id = $1 AND t.owner_user_id = $1 AND lower(t.name) = lower($2)
                  AND b.deleted_at IS NULL AND b.is_vault = FALSE
                ORDER BY sort_title, b.id
                LIMIT $3
                "#,
            )
            .bind(user_id)
            .bind(tag)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            if rows.is_empty() {
                return Err(Error::NotFound);
            }
            (
                SkillSource {
                    kind: SkillSourceKind::Tag,
                    label,
                },
                rows,
            )
        }
    };

    let mut rows = rows;
    let truncated = if rows.len() > MAX_SKILL_BOOKMARKS {
        rows.truncate(MAX_SKILL_BOOKMARKS);
        count_matches(pool, user_id, from).await? - MAX_SKILL_BOOKMARKS
    } else {
        0
    };

    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let tag_rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT bt.bookmark_id, t.name FROM bookmark_tags bt JOIN tags t ON t.id = bt.tag_id \
         WHERE bt.bookmark_id = ANY($1) ORDER BY lower(t.name)",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let mut tags: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, name) in tag_rows {
        tags.entry(id).or_default().push(name);
    }

    let links = rows
        .into_iter()
        .map(|r| SkillLink {
            tags: tags.remove(&r.id).unwrap_or_default(),
            notes: if r.owner_user_id == user_id {
                r.notes
            } else {
                None
            },
            url: r.url,
            title: r.title,
            description: r.description,
            path: r.path,
        })
        .collect();
    Ok(SkillBookmarks {
        source,
        links,
        truncated,
    })
}

/// How many bookmarks a source matches, for the "left out" line of a capped skill.
async fn count_matches(pool: &PgPool, user_id: Uuid, from: &SkillFrom) -> Result<usize> {
    let n: i64 = match from {
        SkillFrom::Folder(id) => {
            sqlx::query_scalar(
                r#"
            WITH RECURSIVE tree AS (
                SELECT id FROM categories WHERE id = $1
                UNION ALL
                SELECT c.id FROM categories c JOIN tree t ON c.parent_id = t.id
                WHERE c.deleted_at IS NULL AND c.is_vault = FALSE
            )
            SELECT count(DISTINCT b.id) FROM tree t
            JOIN bookmarks b ON b.category_id = t.id OR EXISTS (
                SELECT 1 FROM bookmark_categories bc
                WHERE bc.bookmark_id = b.id AND bc.category_id = t.id)
            WHERE b.deleted_at IS NULL AND b.is_vault = FALSE
              AND (b.owner_user_id = $2 OR b.is_private = FALSE)
            "#,
            )
            .bind(id)
            .bind(user_id)
            .fetch_one(pool)
            .await
        }
        SkillFrom::Tag(tag) => {
            sqlx::query_scalar(
                r#"
            SELECT count(DISTINCT b.id) FROM bookmarks b
            JOIN bookmark_tags bt ON bt.bookmark_id = b.id
            JOIN tags t ON t.id = bt.tag_id
            WHERE b.owner_user_id = $1 AND t.owner_user_id = $1 AND lower(t.name) = lower($2)
              AND b.deleted_at IS NULL AND b.is_vault = FALSE
            "#,
            )
            .bind(user_id)
            .bind(tag.trim())
            .fetch_one(pool)
            .await
        }
    }
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.max(0) as usize)
}
