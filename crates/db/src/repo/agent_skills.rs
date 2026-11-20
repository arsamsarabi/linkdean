//! Bookmarks for an Agent Skill made from a folder (with its subfolders), a tag, or a saved search.

use super::bookmark_page::{list_bookmark_ids_matching, BookmarkPageFilter, BookmarkSort};
use super::saved_searches::get_saved_search;
use super::shares::require_category_read;
use chrono::{Duration, Utc};
use linkdean_core::agent_skill::{SkillLink, SkillSource, SkillSourceKind, MAX_SKILL_BOOKMARKS};
use linkdean_core::{Error, Result, SavedSearchFilters, VaultKeyring};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// What a skill is made from.
#[derive(Debug, Clone)]
pub enum SkillFrom {
    Folder(Uuid),
    Tag(String),
    SavedSearch(Uuid),
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
/// else's "Only me" bookmarks or notes. A tag covers the user's own bookmarks only. A saved
/// search uses live filter matches (vault always excluded); pass `ranked_ids` when Meili already
/// ranked a `q` query.
pub async fn agent_skill_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    from: &SkillFrom,
) -> Result<SkillBookmarks> {
    agent_skill_bookmarks_with_rank(pool, user_id, from, None).await
}

/// Same as [`agent_skill_bookmarks`], with an optional Meili-ranked id list for saved searches
/// that include a text query. Other sources ignore `ranked_ids`.
pub async fn agent_skill_bookmarks_with_rank(
    pool: &PgPool,
    user_id: Uuid,
    from: &SkillFrom,
    ranked_ids: Option<Vec<Uuid>>,
) -> Result<SkillBookmarks> {
    match from {
        SkillFrom::SavedSearch(id) => skill_from_saved_search(pool, user_id, *id, ranked_ids).await,
        SkillFrom::Folder(_) | SkillFrom::Tag(_) => {
            let limit = (MAX_SKILL_BOOKMARKS + 1) as i64;
            let (source, rows) = match from {
                SkillFrom::Folder(id) => folder_rows(pool, user_id, *id, limit).await?,
                SkillFrom::Tag(tag) => tag_rows(pool, user_id, tag, limit).await?,
                SkillFrom::SavedSearch(_) => unreachable!(),
            };
            finish(pool, user_id, source, rows, from).await
        }
    }
}

async fn folder_rows(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    limit: i64,
) -> Result<(SkillSource, Vec<Row>)> {
    let (cat, _) = require_category_read(pool, user_id, id).await?;
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
    Ok((
        SkillSource {
            kind: SkillSourceKind::Folder,
            label: cat.name,
        },
        rows,
    ))
}

async fn tag_rows(
    pool: &PgPool,
    user_id: Uuid,
    tag: &str,
    limit: i64,
) -> Result<(SkillSource, Vec<Row>)> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Err(Error::BadRequest("tag is required".into()));
    }
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
    Ok((
        SkillSource {
            kind: SkillSourceKind::Tag,
            label,
        },
        rows,
    ))
}

async fn skill_from_saved_search(
    pool: &PgPool,
    user_id: Uuid,
    search_id: Uuid,
    ranked_ids: Option<Vec<Uuid>>,
) -> Result<SkillBookmarks> {
    let view = get_saved_search(pool, user_id, search_id).await?;
    let source = SkillSource {
        kind: SkillSourceKind::SavedSearch,
        label: view.name,
    };
    // Vault always excluded from skills, even if the snapshot asked for vault.
    let filters = SavedSearchFilters {
        vault: None,
        vault_id: None,
        ..view.filters
    };
    let page = saved_to_page_filter(&filters)?;

    let mut ids = if let Some(ranked) = ranked_ids {
        filter_id_allowlist(pool, user_id, &page, &filters, &ranked).await?
    } else {
        let mut ids =
            list_bookmark_ids_matching(pool, user_id, page, &VaultKeyring::default()).await?;
        ids = apply_client_categories(pool, &ids, &filters).await?;
        ids
    };

    if ids.is_empty() {
        return Err(Error::NotFound);
    }

    let truncated = if ids.len() > MAX_SKILL_BOOKMARKS {
        let left = ids.len() - MAX_SKILL_BOOKMARKS;
        ids.truncate(MAX_SKILL_BOOKMARKS);
        left
    } else {
        0
    };

    let rows = rows_in_order(pool, &ids).await?;
    finish_rows(pool, user_id, source, rows, truncated).await
}

fn saved_to_page_filter(filters: &SavedSearchFilters) -> Result<BookmarkPageFilter> {
    let folder = match filters.folder.as_deref() {
        Some(f) if !f.is_empty() && f != "all" => Some(f.to_string()),
        _ if filters.category.len() == 1 => Some(filters.category[0].clone()),
        _ => None,
    };
    let since = match filters.since.as_deref() {
        None | Some("") => None,
        Some(s) => Some(since_preset(s)?),
    };
    Ok(BookmarkPageFilter {
        folder,
        tags: filters.tag.clone(),
        flags: filters.flag.clone(),
        since,
        vault_only: false,
        include_vault: false,
        include_deleted: false,
        sort: BookmarkSort::parse(filters.sort.as_deref().unwrap_or("")),
        limit: (MAX_SKILL_BOOKMARKS + 1) as i64,
        cursor: None,
    })
}

fn since_preset(key: &str) -> Result<chrono::DateTime<Utc>> {
    let days = match key {
        "7d" => 7,
        "30d" => 30,
        "90d" => 90,
        "year" => 365,
        _ => return Err(Error::BadRequest("unknown since".into())),
    };
    Ok(Utc::now() - Duration::days(days))
}

/// Mirror the library's client-side category OR when it is not already the SQL folder scope.
fn needs_client_categories(filters: &SavedSearchFilters) -> bool {
    let cats = &filters.category;
    if cats.is_empty() {
        return false;
    }
    let folder = filters
        .folder
        .as_deref()
        .filter(|f| !f.is_empty() && *f != "all");
    match (folder, cats.len()) {
        // Single category became the folder in `saved_to_page_filter`.
        (None, 1) => false,
        _ => true,
    }
}

async fn apply_client_categories(
    pool: &PgPool,
    ids: &[Uuid],
    filters: &SavedSearchFilters,
) -> Result<Vec<Uuid>> {
    if !needs_client_categories(filters) {
        return Ok(ids.to_vec());
    }
    filter_ids_by_categories(pool, ids, &filters.category).await
}

async fn filter_ids_by_categories(
    pool: &PgPool,
    ids: &[Uuid],
    category: &[String],
) -> Result<Vec<Uuid>> {
    if ids.is_empty() || category.is_empty() {
        return Ok(ids.to_vec());
    }
    let want_unsorted = category.iter().any(|c| c == "uncategorized");
    let cat_uuids: Vec<Uuid> = category
        .iter()
        .filter(|c| *c != "uncategorized")
        .filter_map(|c| Uuid::parse_str(c).ok())
        .collect();
    let matched: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT b.id FROM bookmarks b
        WHERE b.id = ANY($1)
          AND (
            ($2 AND b.category_id IS NULL AND NOT EXISTS (
                SELECT 1 FROM bookmark_categories bc WHERE bc.bookmark_id = b.id))
            OR b.category_id = ANY($3)
            OR EXISTS (
                SELECT 1 FROM bookmark_categories bc
                WHERE bc.bookmark_id = b.id AND bc.category_id = ANY($3))
          )
        "#,
    )
    .bind(ids)
    .bind(want_unsorted)
    .bind(&cat_uuids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let set: std::collections::HashSet<Uuid> = matched.into_iter().collect();
    Ok(ids.iter().copied().filter(|id| set.contains(id)).collect())
}

/// Keep Meili order among ids that still match the saved filters (vault already out of Meili).
async fn filter_id_allowlist(
    pool: &PgPool,
    user_id: Uuid,
    page: &BookmarkPageFilter,
    filters: &SavedSearchFilters,
    ranked: &[Uuid],
) -> Result<Vec<Uuid>> {
    if ranked.is_empty() {
        return Ok(Vec::new());
    }
    let matched =
        list_bookmark_ids_matching(pool, user_id, page.clone(), &VaultKeyring::default()).await?;
    let matched = apply_client_categories(pool, &matched, filters).await?;
    let set: std::collections::HashSet<Uuid> = matched.into_iter().collect();
    Ok(ranked
        .iter()
        .copied()
        .filter(|id| set.contains(id))
        .collect())
}

async fn rows_in_order(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<Row>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT b.id, b.owner_user_id, b.url, b.title, b.description, b.notes,
            ARRAY[]::text[] AS path
        FROM bookmarks b
        WHERE b.id = ANY($1)
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let mut by_id: HashMap<Uuid, Row> = rows.into_iter().map(|r| (r.id, r)).collect();
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

async fn finish(
    pool: &PgPool,
    user_id: Uuid,
    source: SkillSource,
    mut rows: Vec<Row>,
    from: &SkillFrom,
) -> Result<SkillBookmarks> {
    let truncated = if rows.len() > MAX_SKILL_BOOKMARKS {
        rows.truncate(MAX_SKILL_BOOKMARKS);
        count_matches(pool, user_id, from).await? - MAX_SKILL_BOOKMARKS
    } else {
        0
    };
    finish_rows(pool, user_id, source, rows, truncated).await
}

async fn finish_rows(
    pool: &PgPool,
    user_id: Uuid,
    source: SkillSource,
    rows: Vec<Row>,
    truncated: usize,
) -> Result<SkillBookmarks> {
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
        SkillFrom::SavedSearch(_) => Ok(0),
    }
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.max(0) as usize)
}
