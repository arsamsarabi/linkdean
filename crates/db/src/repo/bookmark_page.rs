//! Keyset-paginated bookmark list for the library / explorer, filtered and paged in SQL.

use super::bookmarks::bookmarks_to_views;
use super::shares::{list_shared_categories, require_category_read};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use linkdean_core::{Bookmark, BookmarkView, Error, Result, VaultKeyring};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

pub const DEFAULT_PAGE_LIMIT: i64 = 50;
pub const MAX_PAGE_LIMIT: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookmarkSort {
    #[default]
    UpdatedDesc,
    UpdatedAsc,
    TitleAsc,
    TitleDesc,
    CreatedDesc,
}

impl BookmarkSort {
    pub fn parse(s: &str) -> Self {
        match s {
            "updated_asc" => Self::UpdatedAsc,
            "title_asc" => Self::TitleAsc,
            "title_desc" => Self::TitleDesc,
            "created_desc" => Self::CreatedDesc,
            _ => Self::UpdatedDesc,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UpdatedDesc => "updated_desc",
            Self::UpdatedAsc => "updated_asc",
            Self::TitleAsc => "title_asc",
            Self::TitleDesc => "title_desc",
            Self::CreatedDesc => "created_desc",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BookmarkPageFilter {
    /// Category UUID, `uncategorized`, or `None` = whole library (owned + shared).
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub flags: Vec<String>,
    pub since: Option<DateTime<Utc>>,
    pub vault_only: bool,
    pub include_vault: bool,
    pub include_deleted: bool,
    pub sort: BookmarkSort,
    pub limit: i64,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CursorPayload {
    /// Sort key that produced this cursor.
    k: String,
    /// Pinned bit (1 = pinned) for pin-first ordering.
    p: i32,
    /// Sort value: RFC 3339 timestamp, or the title key exactly as SQL computed it.
    s: String,
    i: Uuid,
    /// Total matches from the first page, so later pages skip the count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    t: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BookmarkPage {
    pub items: Vec<BookmarkView>,
    pub next_cursor: Option<String>,
    pub total: i64,
}

pub fn encode_cursor(sort: BookmarkSort, pinned: bool, sort_val: &str, id: Uuid) -> String {
    encode_cursor_with_total(sort, pinned, sort_val, id, None)
}

fn encode_cursor_with_total(
    sort: BookmarkSort,
    pinned: bool,
    sort_val: &str,
    id: Uuid,
    total: Option<i64>,
) -> String {
    let payload = CursorPayload {
        k: sort.as_str().into(),
        p: if pinned { 1 } else { 0 },
        s: sort_val.into(),
        i: id,
        t: total,
    };
    let json = serde_json::to_vec(&payload).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(json)
}

fn decode_cursor(raw: &str, expected: BookmarkSort) -> Result<CursorPayload> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.trim())
        .map_err(|_| Error::BadRequest("invalid cursor".into()))?;
    let payload: CursorPayload =
        serde_json::from_slice(&bytes).map_err(|_| Error::BadRequest("invalid cursor".into()))?;
    if payload.k != expected.as_str() {
        return Err(Error::BadRequest("cursor sort mismatch".into()));
    }
    Ok(payload)
}

fn clamp_limit(n: i64) -> i64 {
    if n < 1 {
        DEFAULT_PAGE_LIMIT
    } else {
        n.min(MAX_PAGE_LIMIT)
    }
}

/// Pinned for the viewer first, as 1 or 0.
const PINNED_SQL: &str = "(CASE WHEN COALESCE(f.is_pinned, FALSE) THEN 1 ELSE 0 END)";
/// Titles sort case-insensitively by byte order, falling back to the URL when empty.
const TITLE_SQL: &str = "lower(COALESCE(NULLIF(b.title, ''), b.url)) COLLATE \"C\"";

impl BookmarkSort {
    fn key_sql(self) -> &'static str {
        match self {
            Self::TitleAsc | Self::TitleDesc => TITLE_SQL,
            Self::CreatedDesc => "b.created_at",
            Self::UpdatedAsc | Self::UpdatedDesc => "b.updated_at",
        }
    }

    fn descending(self) -> bool {
        !matches!(self, Self::TitleAsc | Self::UpdatedAsc)
    }

    fn is_title(self) -> bool {
        matches!(self, Self::TitleAsc | Self::TitleDesc)
    }
}

/// Which bookmarks a list covers before the filters.
enum Scope {
    /// The viewer's own bookmarks with no folder.
    Uncategorized,
    /// One folder the viewer can read, owned or shared.
    Folder(Uuid),
    /// The viewer's own bookmarks plus those in folders shared with them.
    Library { shared: Vec<Uuid> },
}

/// `FROM … WHERE …` for a scope and filter. Every value is a bound parameter.
fn push_from_where(
    qb: &mut QueryBuilder<Postgres>,
    user_id: Uuid,
    scope: &Scope,
    filter: &BookmarkPageFilter,
    include_vault: bool,
) {
    qb.push(
        " FROM bookmarks b LEFT JOIN bookmark_flags f ON f.bookmark_id = b.id AND f.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(" WHERE ");
    let push_own_visibility = |qb: &mut QueryBuilder<Postgres>| {
        qb.push("(");
        qb.push_bind(include_vault);
        qb.push(" OR b.is_vault = FALSE) AND (");
        qb.push_bind(filter.include_deleted);
        qb.push(" OR b.deleted_at IS NULL)");
    };
    match scope {
        Scope::Uncategorized => {
            qb.push("b.owner_user_id = ");
            qb.push_bind(user_id);
            qb.push(
                " AND b.category_id IS NULL AND NOT EXISTS (\
                 SELECT 1 FROM bookmark_categories bc WHERE bc.bookmark_id = b.id) AND ",
            );
            push_own_visibility(qb);
        }
        Scope::Folder(cid) => {
            qb.push("(b.category_id = ");
            qb.push_bind(*cid);
            qb.push(
                " OR EXISTS (SELECT 1 FROM bookmark_categories bc \
                 WHERE bc.bookmark_id = b.id AND bc.category_id = ",
            );
            qb.push_bind(*cid);
            qb.push(")) AND ");
            push_own_visibility(qb);
            // Someone else's deleted bookmark stays in their Trash.
            qb.push(" AND (b.deleted_at IS NULL OR b.owner_user_id = ");
            qb.push_bind(user_id);
            qb.push(")");
        }
        Scope::Library { shared } => {
            qb.push("((b.owner_user_id = ");
            qb.push_bind(user_id);
            qb.push(" AND ");
            push_own_visibility(qb);
            qb.push(")");
            if !shared.is_empty() && !filter.vault_only {
                qb.push(" OR (b.owner_user_id <> ");
                qb.push_bind(user_id);
                qb.push(
                    " AND b.is_vault = FALSE AND b.deleted_at IS NULL AND (b.category_id = ANY(",
                );
                qb.push_bind(shared.clone());
                qb.push(
                    ") OR EXISTS (SELECT 1 FROM bookmark_categories bc \
                     WHERE bc.bookmark_id = b.id AND bc.category_id = ANY(",
                );
                qb.push_bind(shared.clone());
                qb.push("))))");
            }
            qb.push(")");
        }
    }
    if filter.vault_only {
        qb.push(" AND b.is_vault = TRUE");
    }
    if let Some(since) = filter.since {
        qb.push(" AND b.created_at >= ");
        qb.push_bind(since);
    }
    for flag in &filter.flags {
        match flag.as_str() {
            "favorite" => qb.push(" AND COALESCE(f.is_favorite, FALSE)"),
            "unread" => qb.push(" AND COALESCE(f.is_unread, FALSE)"),
            "pinned" => qb.push(" AND COALESCE(f.is_pinned, FALSE)"),
            _ => qb,
        };
    }
    let mut tags: Vec<String> = filter.tags.clone();
    tags.sort();
    tags.dedup();
    if !tags.is_empty() {
        // Every requested tag must be on the bookmark.
        let wanted = tags.len() as i64;
        qb.push(
            " AND (SELECT count(DISTINCT t.name) FROM bookmark_tags bt \
             JOIN tags t ON t.id = bt.tag_id WHERE bt.bookmark_id = b.id AND t.name = ANY(",
        );
        qb.push_bind(tags);
        qb.push(")) = ");
        qb.push_bind(wanted);
    }
}

/// Rows after the cursor, in pin-first then sort order, ties broken by id the same way.
fn push_keyset(
    qb: &mut QueryBuilder<Postgres>,
    sort: BookmarkSort,
    cursor: &CursorPayload,
) -> Result<()> {
    let cmp = if sort.descending() { "<" } else { ">" };
    qb.push(" AND (");
    qb.push(PINNED_SQL);
    qb.push(" < ");
    qb.push_bind(cursor.p);
    qb.push(" OR (");
    qb.push(PINNED_SQL);
    qb.push(" = ");
    qb.push_bind(cursor.p);
    qb.push(" AND (");
    qb.push(sort.key_sql());
    qb.push(", b.id) ");
    qb.push(cmp);
    qb.push(" (");
    if sort.is_title() {
        qb.push_bind(cursor.s.clone());
    } else {
        let at = DateTime::parse_from_rfc3339(&cursor.s)
            .map_err(|_| Error::BadRequest("invalid cursor".into()))?
            .with_timezone(&Utc);
        qb.push_bind(at);
    }
    qb.push(", ");
    qb.push_bind(cursor.i);
    qb.push(")))");
    Ok(())
}

fn push_order(qb: &mut QueryBuilder<Postgres>, sort: BookmarkSort) {
    let dir = if sort.descending() { "DESC" } else { "ASC" };
    qb.push(" ORDER BY ");
    qb.push(PINNED_SQL);
    qb.push(" DESC, ");
    qb.push(sort.key_sql());
    qb.push(" ");
    qb.push(dir);
    qb.push(", b.id ");
    qb.push(dir);
}

#[derive(sqlx::FromRow)]
struct PageRow {
    #[sqlx(flatten)]
    bookmark: Bookmark,
    page_pinned: i32,
    page_title: String,
}

/// Checks folder access and resolves the scope and the vault switch for a filter.
async fn resolve(
    pool: &PgPool,
    user_id: Uuid,
    filter: &BookmarkPageFilter,
    keyring: &VaultKeyring,
) -> Result<(Scope, bool)> {
    let include_vault = if filter.vault_only {
        !keyring.is_empty()
    } else {
        filter.include_vault && !keyring.is_empty()
    };
    let scope = match filter.folder.as_deref() {
        Some("uncategorized") => Scope::Uncategorized,
        Some(fid) if fid != "all" => {
            let cid =
                Uuid::parse_str(fid).map_err(|_| Error::BadRequest("invalid folder".into()))?;
            require_category_read(pool, user_id, cid).await?;
            Scope::Folder(cid)
        }
        _ => {
            let shared = list_shared_categories(pool, user_id, true).await?;
            Scope::Library {
                shared: shared.into_iter().map(|(c, _)| c.id).collect(),
            }
        }
    };
    Ok((scope, include_vault))
}

/// Paginated library list. Filters, pin-first sort and the keyset page all run in SQL, so
/// a page costs the same however large the library is; views decrypt via `keyring`.
pub async fn list_bookmarks_page(
    pool: &PgPool,
    user_id: Uuid,
    filter: BookmarkPageFilter,
    keyring: &VaultKeyring,
) -> Result<BookmarkPage> {
    let limit = clamp_limit(filter.limit);
    let cursor = match filter.cursor.as_deref() {
        Some(c) if !c.is_empty() => Some(decode_cursor(c, filter.sort)?),
        _ => None,
    };
    let (scope, include_vault) = resolve(pool, user_id, &filter, keyring).await?;

    let mut qb = QueryBuilder::<Postgres>::new("SELECT b.*, ");
    qb.push(PINNED_SQL);
    qb.push(" AS page_pinned, ");
    qb.push(TITLE_SQL);
    qb.push(" AS page_title");
    push_from_where(&mut qb, user_id, &scope, &filter, include_vault);
    if let Some(c) = cursor.as_ref() {
        push_keyset(&mut qb, filter.sort, c)?;
    }
    push_order(&mut qb, filter.sort);
    qb.push(" LIMIT ");
    qb.push_bind(limit + 1);
    let mut rows: Vec<PageRow> = qb
        .build_query_as()
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let total = match cursor.as_ref().and_then(|c| c.t) {
        Some(t) => t,
        None => {
            let mut count = QueryBuilder::<Postgres>::new("SELECT count(*)");
            push_from_where(&mut count, user_id, &scope, &filter, include_vault);
            count
                .build_query_scalar::<i64>()
                .fetch_one(pool)
                .await
                .map_err(|e| Error::msg(e.to_string()))?
        }
    };

    let has_more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    let next_cursor = if has_more {
        rows.last().map(|r| {
            let value = match filter.sort {
                BookmarkSort::TitleAsc | BookmarkSort::TitleDesc => r.page_title.clone(),
                BookmarkSort::CreatedDesc => r.bookmark.created_at.to_rfc3339(),
                BookmarkSort::UpdatedAsc | BookmarkSort::UpdatedDesc => {
                    r.bookmark.updated_at.to_rfc3339()
                }
            };
            encode_cursor_with_total(
                filter.sort,
                r.page_pinned == 1,
                &value,
                r.bookmark.id,
                Some(total),
            )
        })
    } else {
        None
    };

    let none = VaultKeyring::default();
    let views = bookmarks_to_views(
        pool,
        rows.into_iter().map(|r| r.bookmark).collect(),
        user_id,
        |b| {
            if b.is_vault && b.owner_user_id == user_id {
                keyring
            } else {
                &none
            }
        },
    )
    .await?;

    Ok(BookmarkPage {
        items: views,
        next_cursor,
        total,
    })
}

/// Most ids `list_bookmark_ids_matching` returns, for select-all bulk actions.
pub const MAX_MATCHING_IDS: i64 = 10_000;

/// IDs matching the same filters, in list order (for select-all bulk). Caps at 10_000.
pub async fn list_bookmark_ids_matching(
    pool: &PgPool,
    user_id: Uuid,
    filter: BookmarkPageFilter,
    keyring: &VaultKeyring,
) -> Result<Vec<Uuid>> {
    let (scope, include_vault) = resolve(pool, user_id, &filter, keyring).await?;
    let mut qb = QueryBuilder::<Postgres>::new("SELECT b.id");
    push_from_where(&mut qb, user_id, &scope, &filter, include_vault);
    push_order(&mut qb, filter.sort);
    qb.push(" LIMIT ");
    qb.push_bind(MAX_MATCHING_IDS);
    qb.build_query_scalar::<Uuid>()
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

/// Live bookmarks per folder the person can open, and in their Unsorted.
#[derive(Debug, Clone, Default, Serialize)]
pub struct FolderCounts {
    /// Direct bookmarks per folder (not counting subfolders). Folders with none are absent.
    pub folders: std::collections::HashMap<Uuid, i64>,
    pub uncategorized: i64,
}

/// Counts for Explorer's folder tiles in two aggregate queries, without loading bookmarks.
/// A folder counts the bookmarks the person would see in it: not deleted, and no one
/// else's vault items. Unsorted leaves vault items out, as the list does.
pub async fn folder_bookmark_counts(pool: &PgPool, user_id: Uuid) -> Result<FolderCounts> {
    let shared: Vec<Uuid> = list_shared_categories(pool, user_id, true)
        .await?
        .into_iter()
        .map(|(c, _)| c.id)
        .collect();
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        r#"
        WITH visible AS (
            SELECT id FROM categories WHERE owner_user_id = $1 AND deleted_at IS NULL
            UNION
            SELECT unnest($2::uuid[])
        ),
        placed AS (
            SELECT b.id AS bookmark_id, b.category_id AS category_id
            FROM bookmarks b
            WHERE b.category_id IS NOT NULL
              AND b.deleted_at IS NULL
              AND (b.owner_user_id = $1 OR b.is_vault = FALSE)
            UNION
            SELECT bc.bookmark_id, bc.category_id
            FROM bookmark_categories bc
            JOIN bookmarks b ON b.id = bc.bookmark_id
            WHERE b.deleted_at IS NULL
              AND (b.owner_user_id = $1 OR b.is_vault = FALSE)
        )
        SELECT p.category_id, count(*)
        FROM placed p
        JOIN visible v ON v.id = p.category_id
        GROUP BY p.category_id
        "#,
    )
    .bind(user_id)
    .bind(&shared)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let uncategorized: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*) FROM bookmarks b
        WHERE b.owner_user_id = $1
          AND b.deleted_at IS NULL
          AND b.is_vault = FALSE
          AND b.category_id IS NULL
          AND NOT EXISTS (SELECT 1 FROM bookmark_categories bc WHERE bc.bookmark_id = b.id)
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(FolderCounts {
        folders: rows.into_iter().collect(),
        uncategorized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        let id = Uuid::nil();
        let enc = encode_cursor(BookmarkSort::UpdatedDesc, true, "2026-01-01T00:00:00Z", id);
        let dec = decode_cursor(&enc, BookmarkSort::UpdatedDesc).unwrap();
        assert_eq!(dec.p, 1);
        assert_eq!(dec.i, id);
        assert_eq!(dec.t, None);
        let with_total = encode_cursor_with_total(BookmarkSort::TitleAsc, false, "a", id, Some(7));
        assert_eq!(
            decode_cursor(&with_total, BookmarkSort::TitleAsc)
                .unwrap()
                .t,
            Some(7)
        );
    }
}
