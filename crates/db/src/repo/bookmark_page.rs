//! Keyset-paginated bookmark list for the library / explorer.

use super::bookmarks::bookmarks_to_views;
use super::shares::{list_shared_categories, require_category_read};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use linkdean_core::{Bookmark, BookmarkView, Error, Result, VaultKeyring};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    /// Sort value: ISO timestamp or lowercased title.
    s: String,
    i: Uuid,
}

#[derive(Debug, Clone)]
pub struct BookmarkPage {
    pub items: Vec<BookmarkView>,
    pub next_cursor: Option<String>,
    pub total: i64,
}

pub fn encode_cursor(sort: BookmarkSort, pinned: bool, sort_val: &str, id: Uuid) -> String {
    let payload = CursorPayload {
        k: sort.as_str().into(),
        p: if pinned { 1 } else { 0 },
        s: sort_val.into(),
        i: id,
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

fn sort_value_of(b: &BookmarkView, sort: BookmarkSort) -> String {
    match sort {
        BookmarkSort::TitleAsc | BookmarkSort::TitleDesc => {
            let t = if b.title.is_empty() { &b.url } else { &b.title };
            t.to_lowercase()
        }
        BookmarkSort::CreatedDesc => b.created_at.to_rfc3339(),
        BookmarkSort::UpdatedAsc | BookmarkSort::UpdatedDesc => b.updated_at.to_rfc3339(),
    }
}

/// Paginated library list. Filters and sort run in SQL; views decrypt via `keyring`.
pub async fn list_bookmarks_page(
    pool: &PgPool,
    user_id: Uuid,
    filter: BookmarkPageFilter,
    keyring: &VaultKeyring,
) -> Result<BookmarkPage> {
    let limit = clamp_limit(filter.limit);
    let include_vault = if filter.vault_only {
        !keyring.is_empty()
    } else {
        filter.include_vault && !keyring.is_empty()
    };

    if let Some(folder) = filter.folder.as_deref() {
        if folder != "uncategorized" && folder != "all" {
            let cid =
                Uuid::parse_str(folder).map_err(|_| Error::BadRequest("invalid folder".into()))?;
            require_category_read(pool, user_id, cid).await?;
        }
    }

    let shared = list_shared_categories(pool, user_id, true).await?;
    let shared_ids: Vec<Uuid> = shared.into_iter().map(|(c, _)| c.id).collect();

    let cursor = match filter.cursor.as_deref() {
        Some(c) if !c.is_empty() => Some(decode_cursor(c, filter.sort)?),
        _ => None,
    };

    // Fetch a larger candidate set when shared folders exist and we must merge in app
    // for folder=all — still better than unbounded. Folder-scoped queries stay in SQL.
    let fetch_limit = limit + 1;

    let (rows, total) = match filter.folder.as_deref() {
        Some("uncategorized") => {
            page_owned_uncategorized(
                pool,
                user_id,
                &filter,
                include_vault,
                cursor.as_ref(),
                fetch_limit,
            )
            .await?
        }
        Some(fid) if fid != "all" => {
            let cid =
                Uuid::parse_str(fid).map_err(|_| Error::BadRequest("invalid folder".into()))?;
            page_in_category(
                pool,
                user_id,
                cid,
                &filter,
                include_vault,
                cursor.as_ref(),
                fetch_limit,
            )
            .await?
        }
        _ => {
            page_library(
                pool,
                user_id,
                &shared_ids,
                &filter,
                include_vault,
                cursor.as_ref(),
                fetch_limit,
            )
            .await?
        }
    };

    let has_more = rows.len() as i64 > limit;
    let rows = if has_more {
        rows.into_iter().take(limit as usize).collect()
    } else {
        rows
    };

    let none = VaultKeyring::default();
    let views = bookmarks_to_views(pool, rows, user_id, |b| {
        if b.is_vault && b.owner_user_id == user_id {
            keyring
        } else {
            &none
        }
    })
    .await?;

    let next_cursor = if has_more {
        views.last().map(|b| {
            encode_cursor(
                filter.sort,
                b.is_pinned,
                &sort_value_of(b, filter.sort),
                b.id,
            )
        })
    } else {
        None
    };

    Ok(BookmarkPage {
        items: views,
        next_cursor,
        total,
    })
}

async fn page_in_category(
    pool: &PgPool,
    user_id: Uuid,
    cid: Uuid,
    filter: &BookmarkPageFilter,
    include_vault: bool,
    cursor: Option<&CursorPayload>,
    fetch_limit: i64,
) -> Result<(Vec<Bookmark>, i64)> {
    // Application-side filter after a bounded fetch is wrong for large folders; use SQL.
    // Start with fetch-all-matching then page in Rust for correctness of flags/tags joins —
    // ponytail: for v1 folder pages, load matching rows with SQL filters then keyset in Rust.
    let rows = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT b.* FROM bookmarks b
        WHERE ($2 OR b.is_vault = FALSE)
          AND ($3 OR b.deleted_at IS NULL)
          AND (b.deleted_at IS NULL OR b.owner_user_id = $4)
          AND (
            b.category_id = $1
            OR EXISTS (
              SELECT 1 FROM bookmark_categories bc
              WHERE bc.bookmark_id = b.id AND bc.category_id = $1
            )
          )
          AND ($5::bool = FALSE OR b.is_vault = TRUE)
          AND ($6::timestamptz IS NULL OR b.created_at >= $6)
        "#,
    )
    .bind(cid)
    .bind(include_vault)
    .bind(filter.include_deleted)
    .bind(user_id)
    .bind(filter.vault_only)
    .bind(filter.since)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    filter_sort_page(pool, user_id, rows, filter, cursor, fetch_limit).await
}

async fn page_owned_uncategorized(
    pool: &PgPool,
    user_id: Uuid,
    filter: &BookmarkPageFilter,
    include_vault: bool,
    cursor: Option<&CursorPayload>,
    fetch_limit: i64,
) -> Result<(Vec<Bookmark>, i64)> {
    let rows = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT b.* FROM bookmarks b
        WHERE b.owner_user_id = $1
          AND ($2 OR b.is_vault = FALSE)
          AND ($3 OR b.deleted_at IS NULL)
          AND b.category_id IS NULL
          AND NOT EXISTS (
            SELECT 1 FROM bookmark_categories bc WHERE bc.bookmark_id = b.id
          )
          AND ($4::bool = FALSE OR b.is_vault = TRUE)
          AND ($5::timestamptz IS NULL OR b.created_at >= $5)
        "#,
    )
    .bind(user_id)
    .bind(include_vault)
    .bind(filter.include_deleted)
    .bind(filter.vault_only)
    .bind(filter.since)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    filter_sort_page(pool, user_id, rows, filter, cursor, fetch_limit).await
}

async fn page_library(
    pool: &PgPool,
    user_id: Uuid,
    shared_ids: &[Uuid],
    filter: &BookmarkPageFilter,
    include_vault: bool,
    cursor: Option<&CursorPayload>,
    fetch_limit: i64,
) -> Result<(Vec<Bookmark>, i64)> {
    let mut owned = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT * FROM bookmarks
        WHERE owner_user_id = $1
          AND ($2 OR is_vault = FALSE)
          AND ($3 OR deleted_at IS NULL)
          AND ($4::bool = FALSE OR is_vault = TRUE)
          AND ($5::timestamptz IS NULL OR created_at >= $5)
        "#,
    )
    .bind(user_id)
    .bind(include_vault)
    .bind(filter.include_deleted)
    .bind(filter.vault_only)
    .bind(filter.since)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if !shared_ids.is_empty() && !filter.vault_only {
        let shared_rows = sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT DISTINCT b.* FROM bookmarks b
            WHERE b.owner_user_id <> $1
              AND b.is_vault = FALSE
              AND b.deleted_at IS NULL
              AND ($3::timestamptz IS NULL OR b.created_at >= $3)
              AND (
                b.category_id = ANY($2)
                OR EXISTS (
                  SELECT 1 FROM bookmark_categories bc
                  WHERE bc.bookmark_id = b.id AND bc.category_id = ANY($2)
                )
              )
            "#,
        )
        .bind(user_id)
        .bind(shared_ids)
        .bind(filter.since)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let mut seen: std::collections::HashSet<Uuid> = owned.iter().map(|b| b.id).collect();
        for b in shared_rows {
            if seen.insert(b.id) {
                owned.push(b);
            }
        }
    }

    filter_sort_page(pool, user_id, owned, filter, cursor, fetch_limit).await
}

/// Apply tag/flag filters, pin-first sort, keyset slice. Known issue: loads the filtered
/// set into memory before paging — fine until libraries hit tens of thousands; then push
/// the keyset into SQL (see ROADMAP deferred note if we park that).
async fn filter_sort_page(
    pool: &PgPool,
    user_id: Uuid,
    rows: Vec<Bookmark>,
    filter: &BookmarkPageFilter,
    cursor: Option<&CursorPayload>,
    fetch_limit: i64,
) -> Result<(Vec<Bookmark>, i64)> {
    let ids: Vec<Uuid> = rows.iter().map(|b| b.id).collect();
    let flags = load_flags_map(pool, user_id, &ids).await?;
    let tags = if filter.tags.is_empty() {
        None
    } else {
        Some(load_tags_map(pool, &ids).await?)
    };

    let mut matched: Vec<(Bookmark, bool)> = Vec::new();
    for b in rows {
        let f = flags.get(&b.id).copied().unwrap_or_default();
        if !flags_match(&filter.flags, f) {
            continue;
        }
        if let Some(ref tag_map) = tags {
            let have = tag_map.get(&b.id).cloned().unwrap_or_default();
            if !filter.tags.iter().all(|t| have.iter().any(|h| h == t)) {
                continue;
            }
        }
        matched.push((b, f.2));
    }

    let total = matched.len() as i64;
    matched.sort_by(|(a, ap), (b, bp)| {
        let apo = if *ap { 1 } else { 0 };
        let bpo = if *bp { 1 } else { 0 };
        bpo.cmp(&apo).then_with(|| cmp_bookmarks(a, b, filter.sort))
    });

    let start = if let Some(c) = cursor {
        matched
            .iter()
            .position(|(b, pinned)| after_cursor(b, *pinned, c, filter.sort))
            .unwrap_or(matched.len())
    } else {
        0
    };

    let page: Vec<Bookmark> = matched
        .into_iter()
        .skip(start)
        .take(fetch_limit as usize)
        .map(|(b, _)| b)
        .collect();

    Ok((page, total))
}

/// True when `b` is strictly after the cursor in pin-first + sort order.
fn after_cursor(b: &Bookmark, pinned: bool, c: &CursorPayload, sort: BookmarkSort) -> bool {
    let p = if pinned { 1 } else { 0 };
    match p.cmp(&c.p) {
        // Higher pin comes first; lower pin is after.
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            let sv = bookmark_sort_val(b, sort);
            match sort {
                BookmarkSort::TitleAsc | BookmarkSort::UpdatedAsc => {
                    (sv.as_str(), b.id) > (c.s.as_str(), c.i)
                }
                BookmarkSort::TitleDesc | BookmarkSort::UpdatedDesc | BookmarkSort::CreatedDesc => {
                    (sv.as_str(), b.id) < (c.s.as_str(), c.i)
                }
            }
        }
    }
}

fn bookmark_sort_val(b: &Bookmark, sort: BookmarkSort) -> String {
    match sort {
        BookmarkSort::TitleAsc | BookmarkSort::TitleDesc => {
            let t = if b.title.is_empty() { &b.url } else { &b.title };
            t.to_lowercase()
        }
        BookmarkSort::CreatedDesc => b.created_at.to_rfc3339(),
        BookmarkSort::UpdatedAsc | BookmarkSort::UpdatedDesc => b.updated_at.to_rfc3339(),
    }
}

fn cmp_bookmarks(a: &Bookmark, b: &Bookmark, sort: BookmarkSort) -> std::cmp::Ordering {
    match sort {
        BookmarkSort::TitleAsc => a
            .title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then(a.id.cmp(&b.id)),
        BookmarkSort::TitleDesc => b
            .title
            .to_lowercase()
            .cmp(&a.title.to_lowercase())
            .then(a.id.cmp(&b.id)),
        BookmarkSort::UpdatedAsc => a.updated_at.cmp(&b.updated_at).then(a.id.cmp(&b.id)),
        BookmarkSort::CreatedDesc => b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)),
        BookmarkSort::UpdatedDesc => b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)),
    }
}

fn flags_match(want: &[String], f: (bool, bool, bool)) -> bool {
    let (fav, unread, pinned) = f;
    want.iter().all(|w| match w.as_str() {
        "favorite" => fav,
        "unread" => unread,
        "pinned" => pinned,
        _ => true,
    })
}

async fn load_flags_map(
    pool: &PgPool,
    user_id: Uuid,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, (bool, bool, bool)>> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let rows: Vec<(Uuid, bool, bool, bool)> = sqlx::query_as(
        r#"
        SELECT bookmark_id, is_favorite, is_unread, is_pinned
        FROM bookmark_flags
        WHERE user_id = $1 AND bookmark_id = ANY($2)
        "#,
    )
    .bind(user_id)
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(id, a, b, c)| (id, (a, b, c)))
        .collect())
}

async fn load_tags_map(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<String>>> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT bt.bookmark_id, t.name
        FROM bookmark_tags bt
        JOIN tags t ON t.id = bt.tag_id
        WHERE bt.bookmark_id = ANY($1)
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let mut map: std::collections::HashMap<Uuid, Vec<String>> = Default::default();
    for (id, name) in rows {
        map.entry(id).or_default().push(name);
    }
    Ok(map)
}

/// IDs matching the same filters (for select-all bulk). Caps at 10_000.
pub async fn list_bookmark_ids_matching(
    pool: &PgPool,
    user_id: Uuid,
    filter: BookmarkPageFilter,
    keyring: &VaultKeyring,
) -> Result<Vec<Uuid>> {
    let page = list_bookmarks_page(
        pool,
        user_id,
        BookmarkPageFilter {
            limit: MAX_PAGE_LIMIT,
            cursor: None,
            ..filter.clone()
        },
        keyring,
    )
    .await?;
    // Walk pages until done or 10k.
    let mut ids: Vec<Uuid> = page.items.iter().map(|b| b.id).collect();
    let mut cursor = page.next_cursor;
    while let Some(c) = cursor {
        if ids.len() >= 10_000 {
            break;
        }
        let next = list_bookmarks_page(
            pool,
            user_id,
            BookmarkPageFilter {
                limit: MAX_PAGE_LIMIT,
                cursor: Some(c),
                ..filter.clone()
            },
            keyring,
        )
        .await?;
        ids.extend(next.items.iter().map(|b| b.id));
        cursor = next.next_cursor;
    }
    ids.truncate(10_000);
    Ok(ids)
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
    }
}
