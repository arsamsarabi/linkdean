//! Suggestions for filing an Unsorted bookmark: what the person already uses on that site.

use linkdean_core::{Error, Result};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct FolderSuggestion {
    pub id: Uuid,
    /// Bookmarks on the same site in this folder.
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct TagSuggestion {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FilingSuggestions {
    pub folders: Vec<FolderSuggestion>,
    pub tags: Vec<TagSuggestion>,
}

const MAX_FOLDERS: i64 = 5;
const MAX_TAGS: i64 = 8;

/// The host of a URL in lower case, without a leading `www.`; NULL when there is none.
const HOST_SQL: &str = r#"lower(regexp_replace(
    substring(url from '^[A-Za-z][A-Za-z0-9+.-]*://(?:[^@/?#]*@)?([^/:?#]+)'),
    '^www\.', ''))"#;

/// Folders and tags on the owner's other live, non-vault bookmarks with the same host as
/// `bookmark_id`, most used first. `NotFound` unless `user_id` owns the bookmark.
pub async fn filing_suggestions(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
) -> Result<FilingSuggestions> {
    let found: Option<(bool, Option<String>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT is_vault, {HOST_SQL} FROM bookmarks WHERE id = $1 AND owner_user_id = $2"
    )))
    .bind(bookmark_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let Some((is_vault, host)) = found else {
        return Err(Error::NotFound);
    };
    let Some(host) = host.filter(|h| !h.is_empty()) else {
        return Ok(FilingSuggestions::default());
    };
    if is_vault {
        return Ok(FilingSuggestions::default());
    }

    let peers = format!(
        r#"
        SELECT id, category_id FROM bookmarks
        WHERE owner_user_id = $1 AND id <> $2 AND deleted_at IS NULL AND is_vault = FALSE
          AND {HOST_SQL} = $3
        "#
    );
    let folders: Vec<FolderSuggestion> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        WITH peers AS ({peers}),
        links AS (
            SELECT p.id AS bookmark_id, p.category_id AS category_id FROM peers p
            WHERE p.category_id IS NOT NULL
            UNION
            SELECT bc.bookmark_id, bc.category_id FROM bookmark_categories bc
            JOIN peers p ON p.id = bc.bookmark_id
        )
        SELECT c.id, COUNT(*)::bigint AS count
        FROM links l
        JOIN categories c ON c.id = l.category_id
        WHERE c.owner_user_id = $1 AND c.deleted_at IS NULL AND c.is_vault = FALSE AND c.vault_id IS NULL
        GROUP BY c.id, c.name
        ORDER BY count DESC, c.name ASC
        LIMIT {MAX_FOLDERS}
        "#
    )))
    .bind(user_id)
    .bind(bookmark_id)
    .bind(&host)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let tags: Vec<TagSuggestion> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r#"
        WITH peers AS ({peers})
        SELECT t.name, COUNT(*)::bigint AS count
        FROM peers p
        JOIN bookmark_tags bt ON bt.bookmark_id = p.id
        JOIN tags t ON t.id = bt.tag_id
        GROUP BY t.name
        ORDER BY count DESC, t.name ASC
        LIMIT {MAX_TAGS}
        "#
    )))
    .bind(user_id)
    .bind(bookmark_id)
    .bind(&host)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok(FilingSuggestions { folders, tags })
}
