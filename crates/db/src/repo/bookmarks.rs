use linkdean_core::crypto;
use linkdean_core::VaultKeyring;
use linkdean_core::{
    Bookmark, BookmarkView, Category, CategoryRef, CreateBookmark, Error, LinkCheckStatus, Result,
    UpdateBookmark, VaultPayload,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use super::categories::{get_category_by_id, list_owned_categories};
use super::shares::{
    bookmark_can_read, bookmark_can_write, bookmarks_shared_with, category_access,
    require_category_read, require_category_write,
};
use super::tags::ensure_tags;

pub(super) async fn bookmark_tag_names(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT t.name FROM tags t
        JOIN bookmark_tags bt ON bt.tag_id = t.id
        WHERE bt.bookmark_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Postgres refuses NUL in text. Checked before any write, so a bad field cannot fail a
/// save half way.
fn reject_nul<'a>(fields: impl IntoIterator<Item = &'a str>) -> Result<()> {
    if fields.into_iter().any(|f| f.contains('\0')) {
        return Err(Error::BadRequest(
            "text cannot contain NUL characters".into(),
        ));
    }
    Ok(())
}

fn reject_nul_in_update(req: &UpdateBookmark) -> Result<()> {
    fn opt(o: &Option<Option<String>>) -> Option<&str> {
        o.as_ref().and_then(|v| v.as_deref())
    }
    reject_nul(
        req.url
            .as_deref()
            .into_iter()
            .chain(req.title.as_deref())
            .chain(opt(&req.description))
            .chain(opt(&req.notes))
            .chain(req.tags.iter().flatten().map(String::as_str)),
    )
}

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// Replaces a bookmark's tags. Tags are made beforehand with `ensure_tags`.
async fn apply_tags(
    conn: &mut sqlx::PgConnection,
    bookmark_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<()> {
    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = $1")
        .bind(bookmark_id)
        .execute(&mut *conn)
        .await
        .map_err(db_err)?;
    sqlx::query(
        r#"
        INSERT INTO bookmark_tags (bookmark_id, tag_id)
        SELECT $1, t FROM unnest($2::uuid[]) AS t
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(bookmark_id)
    .bind(tag_ids)
    .execute(&mut *conn)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, sqlx::FromRow)]
struct BookmarkFlagsRow {
    is_favorite: bool,
    is_unread: bool,
    is_pinned: bool,
}

async fn flags_for_viewer(
    pool: &PgPool,
    viewer_id: Uuid,
    bookmark_id: Uuid,
) -> Result<BookmarkFlagsRow> {
    let row = sqlx::query_as::<_, BookmarkFlagsRow>(
        r#"
        SELECT is_favorite, is_unread, is_pinned
        FROM bookmark_flags
        WHERE user_id = $1 AND bookmark_id = $2
        "#,
    )
    .bind(viewer_id)
    .bind(bookmark_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.unwrap_or_default())
}

/// Upsert per-viewer flags. Any reader may set their own flags.
async fn set_bookmark_flags(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    is_favorite: Option<bool>,
    is_unread: Option<bool>,
    is_pinned: Option<bool>,
) -> Result<()> {
    if !bookmark_can_read(pool, user_id, bookmark_id).await? {
        return Err(Error::NotFound);
    }
    let current = flags_for_viewer(pool, user_id, bookmark_id).await?;
    let next = BookmarkFlagsRow {
        is_favorite: is_favorite.unwrap_or(current.is_favorite),
        is_unread: is_unread.unwrap_or(current.is_unread),
        is_pinned: is_pinned.unwrap_or(current.is_pinned),
    };
    if !next.is_favorite && !next.is_unread && !next.is_pinned {
        sqlx::query("DELETE FROM bookmark_flags WHERE user_id = $1 AND bookmark_id = $2")
            .bind(user_id)
            .bind(bookmark_id)
            .execute(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        return Ok(());
    }
    sqlx::query(
        r#"
        INSERT INTO bookmark_flags (user_id, bookmark_id, is_favorite, is_unread, is_pinned)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_id, bookmark_id) DO UPDATE
          SET is_favorite = EXCLUDED.is_favorite,
              is_unread = EXCLUDED.is_unread,
              is_pinned = EXCLUDED.is_pinned,
              updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(bookmark_id)
    .bind(next.is_favorite)
    .bind(next.is_unread)
    .bind(next.is_pinned)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub(crate) async fn tags_for_bookmark(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT t.name FROM tags t
        JOIN bookmark_tags bt ON bt.tag_id = t.id
        WHERE bt.bookmark_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|(n,)| n).collect())
}

fn category_path(cat: &Category, by_id: &HashMap<Uuid, Category>) -> String {
    let mut parts = vec![cat.name.clone()];
    let mut parent = cat.parent_id;
    let mut guard = 0;
    while let Some(pid) = parent {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(p) = by_id.get(&pid) else { break };
        parts.push(p.name.clone());
        parent = p.parent_id;
    }
    parts.reverse();
    parts.join(" / ")
}

/// The categories on a bookmark that `viewer_id` may see. The owner sees all of them with
/// full paths. Anyone else sees only lists they can read, with each path starting at the
/// highest list they can read, so the owner's other folder names never show.
async fn categories_for_bookmark(
    pool: &PgPool,
    b: &Bookmark,
    viewer_id: Uuid,
    batch: &ViewBatch,
) -> Result<Vec<CategoryRef>> {
    let all = categories_for_bookmark_owner(pool, b, batch).await?;
    if viewer_id == b.owner_user_id {
        return Ok(all);
    }
    let mut out = Vec::new();
    for cat_ref in all {
        if category_access(pool, viewer_id, cat_ref.id).await.is_err() {
            continue;
        }
        let path = readable_path(pool, viewer_id, cat_ref.id).await?;
        out.push(CategoryRef { path, ..cat_ref });
    }
    Ok(out)
}

/// Path from the highest ancestor `viewer_id` can read down to `category_id`.
async fn readable_path(pool: &PgPool, viewer_id: Uuid, category_id: Uuid) -> Result<String> {
    let mut parts = Vec::new();
    let mut next = Some(category_id);
    while let Some(id) = next {
        if parts.len() >= 64 || category_access(pool, viewer_id, id).await.is_err() {
            break;
        }
        let Ok(cat) = get_category_by_id(pool, id).await else {
            break;
        };
        parts.push(cat.name);
        next = cat.parent_id;
    }
    parts.reverse();
    Ok(parts.join(" / "))
}

async fn categories_for_bookmark_owner(
    pool: &PgPool,
    b: &Bookmark,
    batch: &ViewBatch,
) -> Result<Vec<CategoryRef>> {
    let Some(linked) = batch.links.get(&b.id) else {
        return Ok(Vec::new());
    };
    let empty = HashMap::new();
    let by_id = batch
        .owner_categories
        .get(&b.owner_user_id)
        .unwrap_or(&empty);
    let mut out = Vec::new();
    for id in linked {
        if let Some(cat) = by_id.get(id) {
            out.push(CategoryRef {
                id: cat.id,
                name: cat.name.clone(),
                path: category_path(cat, by_id),
            });
        } else if let Ok(cat) = get_category_by_id(pool, *id).await {
            // Shared / cross-owner link: path may be shallow if ancestors not loaded.
            out.push(CategoryRef {
                id: cat.id,
                name: cat.name.clone(),
                path: cat.name.clone(),
            });
        }
    }
    Ok(out)
}

/// Tag names per bookmark, sorted, for every bookmark in `ids`.
pub(crate) async fn tags_for_bookmarks(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<String>>> {
    let mut out: HashMap<Uuid, Vec<String>> = HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT bt.bookmark_id, t.name FROM bookmark_tags bt
        JOIN tags t ON t.id = bt.tag_id
        WHERE bt.bookmark_id = ANY($1)
        ORDER BY t.name
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    for (id, name) in rows {
        out.entry(id).or_default().push(name);
    }
    Ok(out)
}

/// What building a set of bookmark views needs, loaded with a few queries for the whole
/// set rather than several for each bookmark.
struct ViewBatch {
    tags: HashMap<Uuid, Vec<String>>,
    flags: HashMap<Uuid, BookmarkFlagsRow>,
    /// Category ids linked to each bookmark, in id order.
    links: HashMap<Uuid, Vec<Uuid>>,
    /// Owned bookmarks only: who their folders are shared with.
    shared_with: HashMap<Uuid, Vec<String>>,
    /// Every category of each owner with a linked bookmark in the set.
    owner_categories: HashMap<Uuid, HashMap<Uuid, Category>>,
    owner_names: HashMap<Uuid, Option<String>>,
}

impl ViewBatch {
    async fn load(pool: &PgPool, rows: &[Bookmark], viewer_id: Uuid) -> Result<Self> {
        let ids: Vec<Uuid> = rows.iter().map(|b| b.id).collect();
        let plain: Vec<Uuid> = rows.iter().filter(|b| !b.is_vault).map(|b| b.id).collect();
        let owned: Vec<Uuid> = rows
            .iter()
            .filter(|b| b.owner_user_id == viewer_id)
            .map(|b| b.id)
            .collect();

        let tags = tags_for_bookmarks(pool, &plain).await?;

        let flag_rows: Vec<(Uuid, bool, bool, bool)> = sqlx::query_as(
            r#"
            SELECT bookmark_id, is_favorite, is_unread, is_pinned
            FROM bookmark_flags
            WHERE user_id = $1 AND bookmark_id = ANY($2)
            "#,
        )
        .bind(viewer_id)
        .bind(&ids)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        let flags = flag_rows
            .into_iter()
            .map(|(id, is_favorite, is_unread, is_pinned)| {
                (
                    id,
                    BookmarkFlagsRow {
                        is_favorite,
                        is_unread,
                        is_pinned,
                    },
                )
            })
            .collect();

        let link_rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            r#"
            SELECT bookmark_id, category_id FROM bookmark_categories
            WHERE bookmark_id = ANY($1)
            ORDER BY bookmark_id, category_id
            "#,
        )
        .bind(&ids)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        let mut links: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (bookmark_id, category_id) in link_rows {
            links.entry(bookmark_id).or_default().push(category_id);
        }

        let shared_with = bookmarks_shared_with(pool, &owned).await?;

        let mut owner_categories = HashMap::new();
        let mut owner_names = HashMap::new();
        for b in rows {
            let owner = b.owner_user_id;
            if !owner_names.contains_key(&owner) {
                let name =
                    super::users::owner_display_name_for_viewer(pool, owner, viewer_id).await?;
                owner_names.insert(owner, name);
            }
            if links.contains_key(&b.id) && !owner_categories.contains_key(&owner) {
                let cats = list_owned_categories(pool, owner, true, true).await?;
                owner_categories.insert(owner, cats.into_iter().map(|c| (c.id, c)).collect());
            }
        }

        Ok(Self {
            tags,
            flags,
            links,
            shared_with,
            owner_categories,
            owner_names,
        })
    }
}

/// Where a bookmark will be filed: every link to write, and the primary category.
struct CategoryPlan {
    links: Vec<Uuid>,
    primary: Option<Uuid>,
}

/// Checks a request to file a bookmark in `category_ids`, and works out the links. Reads
/// only, so it runs before the write transaction opens.
async fn plan_bookmark_categories(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    category_ids: &[Uuid],
) -> Result<CategoryPlan> {
    let mut unique = Vec::new();
    for id in category_ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }

    let (owner_id, bookmark_vault, current_primary): (Uuid, Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT owner_user_id, vault_id, category_id FROM bookmarks WHERE id = $1")
            .bind(bookmark_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    let is_owner = owner_id == user_id;
    // Whose lists the bookmark already sits in. The owner of a list that holds someone
    // else's bookmark may move it within their own lists.
    let tree_owners: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT c.owner_user_id FROM bookmark_categories bc
        JOIN categories c ON c.id = bc.category_id
        WHERE bc.bookmark_id = $1
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    for id in &unique {
        let (cat, _) = require_category_write(pool, user_id, *id).await?;
        if cat.vault_id != bookmark_vault {
            return Err(Error::BadRequest(
                "a bookmark can only sit in categories of its own vault".into(),
            ));
        }
        // A list grants access to what it holds, so someone else's bookmark may only be
        // filed in lists of whoever already holds it. Otherwise an invitee could file it in
        // their own list and keep it after losing access.
        if !is_owner && cat.owner_user_id != owner_id && !tree_owners.contains(&cat.owner_user_id) {
            return Err(Error::Forbidden);
        }
    }

    // Someone else only changes the links they can write. The owner's other lists,
    // including private ones, stay.
    let mut links = unique.clone();
    if !is_owner {
        let existing: Vec<Uuid> = sqlx::query_scalar(
            "SELECT category_id FROM bookmark_categories WHERE bookmark_id = $1",
        )
        .bind(bookmark_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        for id in existing {
            let writable =
                matches!(category_access(pool, user_id, id).await, Ok(a) if a.can_write());
            if !writable && !links.contains(&id) {
                links.push(id);
            }
        }
    }

    let primary = match current_primary {
        Some(p) if !is_owner && links.contains(&p) => Some(p),
        _ => links.first().copied(),
    };
    Ok(CategoryPlan { links, primary })
}

async fn apply_categories(
    conn: &mut sqlx::PgConnection,
    bookmark_id: Uuid,
    plan: &CategoryPlan,
) -> Result<()> {
    sqlx::query("DELETE FROM bookmark_categories WHERE bookmark_id = $1")
        .bind(bookmark_id)
        .execute(&mut *conn)
        .await
        .map_err(db_err)?;
    sqlx::query(
        r#"
        INSERT INTO bookmark_categories (bookmark_id, category_id)
        SELECT $1, c FROM unnest($2::uuid[]) AS c
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(bookmark_id)
    .bind(&plan.links)
    .execute(&mut *conn)
    .await
    .map_err(db_err)?;
    sqlx::query("UPDATE bookmarks SET category_id = $2 WHERE id = $1")
        .bind(bookmark_id)
        .bind(plan.primary)
        .execute(&mut *conn)
        .await
        .map_err(db_err)?;
    Ok(())
}

/// The categories an update asks for, if it asks.
fn requested_categories(req: &UpdateBookmark) -> Option<Vec<Uuid>> {
    match (&req.category_ids, req.category_id) {
        (Some(ids), _) => Some(ids.clone()),
        (None, Some(c)) => Some(c.into_iter().collect()),
        (None, None) => None,
    }
}

fn resolve_category_ids(req_ids: &[Uuid], legacy: Option<Uuid>) -> Vec<Uuid> {
    if !req_ids.is_empty() {
        return req_ids.to_vec();
    }
    legacy.into_iter().collect()
}

pub async fn bookmark_to_view(
    pool: &PgPool,
    b: Bookmark,
    keyring: &VaultKeyring,
    viewer_id: Uuid,
) -> Result<Option<BookmarkView>> {
    let batch = ViewBatch::load(pool, std::slice::from_ref(&b), viewer_id).await?;
    view_from_batch(pool, b, keyring, viewer_id, &batch).await
}

/// Views of `rows` for `viewer_id`, in the same order, leaving out vault bookmarks whose key
/// `keyring_for` does not hold.
pub async fn bookmarks_to_views<'k>(
    pool: &PgPool,
    rows: Vec<Bookmark>,
    viewer_id: Uuid,
    keyring_for: impl Fn(&Bookmark) -> &'k VaultKeyring,
) -> Result<Vec<BookmarkView>> {
    let batch = ViewBatch::load(pool, &rows, viewer_id).await?;
    let mut out = Vec::with_capacity(rows.len());
    for b in rows {
        let keyring = keyring_for(&b);
        if let Some(view) = view_from_batch(pool, b, keyring, viewer_id, &batch).await? {
            out.push(view);
        }
    }
    Ok(out)
}

async fn view_from_batch(
    pool: &PgPool,
    b: Bookmark,
    keyring: &VaultKeyring,
    viewer_id: Uuid,
    batch: &ViewBatch,
) -> Result<Option<BookmarkView>> {
    let categories = categories_for_bookmark(pool, &b, viewer_id, batch).await?;
    let category_id = categories
        .first()
        .map(|c| c.id)
        .or(b.category_id.filter(|_| b.owner_user_id == viewer_id));
    let can_write = if b.owner_user_id == viewer_id {
        true
    } else {
        bookmark_can_write(pool, viewer_id, b.id).await?
    };
    let shared_with = batch.shared_with.get(&b.id).cloned().unwrap_or_default();
    let shared_out = !shared_with.is_empty();
    let owner_display_name = batch.owner_names.get(&b.owner_user_id).cloned().flatten();
    let flags = batch.flags.get(&b.id).copied().unwrap_or_default();

    if b.is_vault {
        let Some(key) = b.vault_id.and_then(|id| keyring.get(id)) else {
            return Ok(None);
        };
        let ct = b
            .vault_ciphertext
            .clone()
            .ok_or(Error::msg("vault blob missing"))?;
        let nonce = b
            .vault_nonce
            .clone()
            .ok_or(Error::msg("vault nonce missing"))?;
        let key = *key;
        let payload =
            super::blocking::run_blocking(move || crypto::decrypt_payload(&key, &ct, &nonce))
                .await?;
        return Ok(Some(BookmarkView {
            id: b.id,
            owner_user_id: b.owner_user_id,
            category_id,
            categories,
            url: payload.url,
            title: payload.title,
            description: payload.description,
            notes: payload.notes,
            favicon_url: b.favicon_url,
            preview_image_url: b.preview_image_url,
            has_preview: b.has_preview,
            is_private: false,
            is_vault: true,
            vault_id: b.vault_id,
            tags: payload.tags,
            can_write,
            shared_out,
            shared_with: shared_with.clone(),
            owner_display_name: owner_display_name.clone(),
            is_favorite: flags.is_favorite,
            is_unread: flags.is_unread,
            is_pinned: flags.is_pinned,
            deleted_at: b.deleted_at,
            link_check_status: None,
            link_check_status_code: None,
            link_check_error: None,
            link_checked_at: None,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }));
    }
    let tags = batch.tags.get(&b.id).cloned().unwrap_or_default();
    let (link_check_status, link_check_status_code, link_check_error, link_checked_at) =
        link_check_fields(&b);
    Ok(Some(BookmarkView {
        id: b.id,
        owner_user_id: b.owner_user_id,
        category_id,
        categories,
        url: b.url,
        title: b.title,
        description: b.description,
        notes: b.notes,
        favicon_url: b.favicon_url,
        preview_image_url: b.preview_image_url,
        has_preview: b.has_preview,
        is_private: b.is_private,
        is_vault: false,
        vault_id: None,
        tags,
        can_write,
        shared_out,
        shared_with,
        owner_display_name,
        is_favorite: flags.is_favorite,
        is_unread: flags.is_unread,
        is_pinned: flags.is_pinned,
        deleted_at: b.deleted_at,
        link_check_status,
        link_check_status_code,
        link_check_error,
        link_checked_at,
        created_at: b.created_at,
        updated_at: b.updated_at,
    }))
}

fn link_check_fields(
    b: &Bookmark,
) -> (
    Option<LinkCheckStatus>,
    Option<i32>,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    let status = b
        .link_check_status
        .as_deref()
        .and_then(|s| LinkCheckStatus::parse(s).ok());
    (
        status,
        b.link_check_status_code,
        b.link_check_error.clone(),
        b.link_checked_at,
    )
}

pub async fn list_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Option<Uuid>,
    include_vault: bool,
    keyring: &VaultKeyring,
    include_deleted: bool,
) -> Result<Vec<BookmarkView>> {
    use super::shares::list_shared_categories;

    let include_vault = include_vault && !keyring.is_empty();
    let rows = if let Some(cid) = category_id {
        require_category_read(pool, user_id, cid).await?;
        sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT b.* FROM bookmarks b
            WHERE ($2 OR b.is_vault = FALSE)
              AND ($3 OR b.deleted_at IS NULL)
              -- Trash is the owner's alone, even in a shared folder.
              AND (b.deleted_at IS NULL OR b.owner_user_id = $4)
              AND (
                b.category_id = $1
                OR EXISTS (
                  SELECT 1 FROM bookmark_categories bc
                  WHERE bc.bookmark_id = b.id AND bc.category_id = $1
                )
              )
            ORDER BY b.updated_at DESC
            "#,
        )
        .bind(cid)
        .bind(include_vault)
        .bind(include_deleted)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
    } else {
        let shared = list_shared_categories(pool, user_id, true).await?;
        let shared_ids: Vec<Uuid> = shared.into_iter().map(|(c, _)| c.id).collect();

        let mut owned = sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT * FROM bookmarks
            WHERE owner_user_id = $1
              AND ($2 OR is_vault = FALSE)
              AND ($3 OR deleted_at IS NULL)
            ORDER BY updated_at DESC
            "#,
        )
        .bind(user_id)
        .bind(include_vault)
        .bind(include_deleted)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        if !shared_ids.is_empty() {
            let shared_rows = sqlx::query_as::<_, Bookmark>(
                r#"
                SELECT DISTINCT b.* FROM bookmarks b
                WHERE b.owner_user_id <> $1
                  AND b.is_vault = FALSE
                  AND b.deleted_at IS NULL
                  AND (
                    b.category_id = ANY($2)
                    OR EXISTS (
                      SELECT 1 FROM bookmark_categories bc
                      WHERE bc.bookmark_id = b.id AND bc.category_id = ANY($2)
                    )
                  )
                ORDER BY b.updated_at DESC
                "#,
            )
            .bind(user_id)
            .bind(&shared_ids)
            .fetch_all(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;

            let mut seen: std::collections::HashSet<Uuid> = owned.iter().map(|b| b.id).collect();
            for b in shared_rows {
                if seen.insert(b.id) {
                    owned.push(b);
                }
            }
            owned.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        }
        owned
    };

    let none = VaultKeyring::default();
    bookmarks_to_views(pool, rows, user_id, |b| {
        if b.is_vault && b.owner_user_id == user_id {
            keyring
        } else {
            &none
        }
    })
    .await
}

pub async fn get_bookmark_raw(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Bookmark> {
    if !bookmark_can_read(pool, user_id, id).await? {
        return Err(Error::NotFound);
    }
    sqlx::query_as::<_, Bookmark>("SELECT * FROM bookmarks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

/// Bookmark for serving its preview, favicon or OG image. A dashboard share shows its
/// non-vault pins to invitees without granting `bookmark_can_read`, so those images
/// are allowed here and nowhere else: this must not back social or write paths.
pub async fn get_bookmark_for_assets(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Bookmark> {
    match get_bookmark_raw(pool, user_id, id).await {
        Err(Error::NotFound) => {}
        result => return result,
    }
    sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT b.*
        FROM bookmarks b
        JOIN dashboard_items di ON di.bookmark_id = b.id
        JOIN dashboard_sections s ON s.id = di.section_id
        JOIN dashboards d ON d.id = s.dashboard_id
        JOIN dashboard_shares ds ON ds.dashboard_id = d.id
        WHERE b.id = $1
          AND ds.invitee_user_id = $2
          AND d.deleted_at IS NULL
          AND b.deleted_at IS NULL
          AND b.is_vault = FALSE
          -- Same rule as the dashboard view: a share carries the owner's own pins only.
          AND b.owner_user_id = d.owner_user_id
        LIMIT 1
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

pub async fn create_bookmark(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateBookmark,
    keyring: &VaultKeyring,
) -> Result<BookmarkView> {
    reject_nul(
        [req.url.as_str()]
            .into_iter()
            .chain(req.title.as_deref())
            .chain(req.description.as_deref())
            .chain(req.notes.as_deref())
            .chain(req.tags.iter().map(String::as_str)),
    )?;
    let cat_ids = resolve_category_ids(&req.category_ids, req.category_id);
    let mut vault_id = req.vault_id;
    let mut first = true;
    for cid in &cat_ids {
        let (cat, access) = require_category_write(pool, user_id, *cid).await?;
        if cat.is_vault && !access.is_owner() {
            return Err(Error::Forbidden);
        }
        if first && req.vault_id.is_none() {
            vault_id = cat.vault_id;
        } else if cat.vault_id != vault_id {
            return Err(Error::BadRequest(
                "a bookmark can only sit in categories of its own vault".into(),
            ));
        }
        first = false;
    }
    if let Some(id) = vault_id {
        super::vault::get_user_vault(pool, user_id, id).await?;
    }

    let title = req.title.clone().unwrap_or_else(|| req.url.clone());
    let mut links: Vec<Uuid> = Vec::new();
    for id in &cat_ids {
        if !links.contains(id) {
            links.push(*id);
        }
    }
    let categories = CategoryPlan {
        primary: links.first().copied(),
        links,
    };
    let primary = categories.primary;

    // Everything that can fail on its own runs before the transaction: checks above,
    // vault encryption and tag creation here.
    let mut sealed = None;
    if let Some(vault_id) = vault_id {
        let key = *keyring.get(vault_id).ok_or(Error::VaultLocked)?;
        let payload = VaultPayload {
            url: req.url.clone(),
            title: title.clone(),
            description: req.description.clone(),
            notes: req.notes.clone(),
            tags: req.tags.clone(),
        };
        sealed = Some(
            super::blocking::run_blocking(move || crypto::encrypt_payload(&key, &payload)).await?,
        );
    }
    let tag_ids: Vec<Uuid> = if vault_id.is_none() {
        ensure_tags(pool, user_id, &req.tags)
            .await?
            .into_iter()
            .map(|t| t.id)
            .collect()
    } else {
        Vec::new()
    };

    let mut tx = pool.begin().await.map_err(db_err)?;
    let bookmark = if let (Some(vault_id), Some((ct, nonce))) = (vault_id, sealed) {
        sqlx::query_as::<_, Bookmark>(
            r#"
            INSERT INTO bookmarks (
                owner_user_id, category_id, url, title, description, notes,
                is_vault, vault_id, vault_ciphertext, vault_nonce
            ) VALUES ($1,$2,'','','',NULL,TRUE,$3,$4,$5)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(primary)
        .bind(vault_id)
        .bind(ct)
        .bind(nonce)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?
    } else {
        let b = sqlx::query_as::<_, Bookmark>(
            r#"
            INSERT INTO bookmarks (
                owner_user_id, category_id, url, title, description, notes, is_vault
            ) VALUES ($1,$2,$3,$4,$5,$6,FALSE)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(primary)
        .bind(&req.url)
        .bind(&title)
        .bind(&req.description)
        .bind(&req.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
        apply_tags(&mut tx, b.id, &tag_ids).await?;
        b
    };

    apply_categories(&mut tx, bookmark.id, &categories).await?;
    if req.is_favorite || req.is_unread || req.is_pinned {
        sqlx::query(
            r#"
            INSERT INTO bookmark_flags (user_id, bookmark_id, is_favorite, is_unread, is_pinned)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(bookmark.id)
        .bind(req.is_favorite)
        .bind(req.is_unread)
        .bind(req.is_pinned)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    let bookmark = get_bookmark_raw(pool, user_id, bookmark.id).await?;
    let view = bookmark_to_view(pool, bookmark, keyring, user_id)
        .await?
        .ok_or(Error::VaultLocked)?;
    let title = activity_bookmark_title(&view);
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Created,
        linkdean_core::ActivitySubjectType::Bookmark,
        view.id,
        &title,
    )
    .await;
    Ok(view)
}

fn activity_bookmark_title(view: &BookmarkView) -> String {
    if view.is_vault {
        return "Vault bookmark".into();
    }
    let t = view.title.trim();
    if t.is_empty() {
        view.url.clone()
    } else {
        view.title.clone()
    }
}

fn update_has_content(req: &UpdateBookmark) -> bool {
    req.url.is_some()
        || req.title.is_some()
        || req.description.is_some()
        || req.notes.is_some()
        || req.category_id.is_some()
        || req.category_ids.is_some()
        || req.tags.is_some()
        || req.is_private.is_some()
}

fn update_has_flags(req: &UpdateBookmark) -> bool {
    req.is_favorite.is_some() || req.is_unread.is_some() || req.is_pinned.is_some()
}

/// `before_title` is `None` for vault rows, whose titles must never reach the activity log.
fn bookmark_content_changes(
    req: &UpdateBookmark,
    before_title: Option<&str>,
) -> (Vec<String>, Vec<&'static str>) {
    let mut changes = Vec::new();
    let mut keys = Vec::new();
    if req.url.is_some() {
        changes.push("url".into());
        keys.push("url");
    }
    if let Some(ref title) = req.title {
        match before_title.map(str::trim) {
            None => changes.push("title".into()),
            Some(prev) if !prev.is_empty() && prev != title.trim() => {
                changes.push(format!("renamed “{prev}” → “{}”", title.trim()));
            }
            Some(_) => changes.push(format!("title set to “{}”", title.trim())),
        }
        keys.push("title");
    }
    if req.description.is_some() {
        changes.push("description".into());
        keys.push("description");
    }
    if req.notes.is_some() {
        changes.push("notes".into());
        keys.push("notes");
    }
    if req.tags.is_some() {
        changes.push("tags".into());
        keys.push("tags");
    }
    if req.category_ids.is_some() || req.category_id.is_some() {
        changes.push("folders".into());
        keys.push("categories");
    }
    (changes, keys)
}

async fn record_bookmark_update_activity(
    pool: &PgPool,
    user_id: Uuid,
    view: &BookmarkView,
    changes: &[String],
    change_keys: &[&str],
) {
    if changes.is_empty() {
        return;
    }
    let title = activity_bookmark_title(view);
    let summary = super::activity::update_summary(
        linkdean_core::ActivitySubjectType::Bookmark,
        &title,
        changes,
    );
    super::activity::record_activity_with(
        pool,
        user_id,
        linkdean_core::ActivityAction::Updated,
        super::activity::ActivitySubject {
            subject_type: linkdean_core::ActivitySubjectType::Bookmark,
            subject_id: view.id,
            subject_title: &title,
        },
        Some(&summary),
        serde_json::json!({ "changes": change_keys }),
    )
    .await;
}

/// Owner-only. Vault bookmarks never appear on a profile, so the flag does not apply to them.
async fn set_bookmark_private(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    is_private: bool,
) -> Result<()> {
    let existing = get_bookmark_raw(pool, user_id, id).await?;
    if existing.owner_user_id != user_id {
        return Err(Error::Forbidden);
    }
    if existing.is_vault {
        return Err(Error::BadRequest(
            "vault bookmarks cannot be marked private".into(),
        ));
    }
    if existing.is_private == is_private {
        return Ok(());
    }
    sqlx::query("UPDATE bookmarks SET is_private = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(is_private)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let change = if is_private {
        "set to Only me"
    } else {
        "visible through its categories"
    };
    let summary = super::activity::update_summary(
        linkdean_core::ActivitySubjectType::Bookmark,
        &existing.title,
        &[change.to_string()],
    );
    super::activity::record_activity_with(
        pool,
        user_id,
        linkdean_core::ActivityAction::Updated,
        super::activity::ActivitySubject {
            subject_type: linkdean_core::ActivitySubjectType::Bookmark,
            subject_id: id,
            subject_title: &existing.title,
        },
        Some(&summary),
        serde_json::json!({ "changes": ["is_private"] }),
    )
    .await;
    Ok(())
}

pub async fn update_bookmark(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateBookmark,
    keyring: &VaultKeyring,
) -> Result<BookmarkView> {
    let content = update_has_content(&req);
    let flags = update_has_flags(&req);
    if !content && !flags {
        return Err(Error::BadRequest("empty update".into()));
    }

    // Flag-only patches: any reader. Content: writers only.
    if content {
        if !bookmark_can_write(pool, user_id, id).await? {
            if bookmark_can_read(pool, user_id, id).await? {
                return Err(Error::Forbidden);
            }
            return Err(Error::NotFound);
        }
    } else if !bookmark_can_read(pool, user_id, id).await? {
        return Err(Error::NotFound);
    }

    // Check everything before writing anything, so a refused request changes nothing.
    reject_nul_in_update(&req)?;
    let existing = get_bookmark_raw(pool, user_id, id).await?;
    if req.is_private.is_some() {
        if existing.owner_user_id != user_id {
            return Err(Error::Forbidden);
        }
        if existing.is_vault {
            return Err(Error::BadRequest(
                "vault bookmarks cannot be marked private".into(),
            ));
        }
    }
    if content && existing.is_vault && existing.owner_user_id != user_id {
        return Err(Error::Forbidden);
    }
    let categories = match requested_categories(&req) {
        Some(ids) if content => Some(plan_bookmark_categories(pool, user_id, id, &ids).await?),
        _ => None,
    };
    // Tags stay in the actor's tag namespace. Vault tags live inside the ciphertext.
    let tag_ids: Option<Vec<Uuid>> = match &req.tags {
        Some(tags) if !existing.is_vault => Some(
            ensure_tags(pool, user_id, tags)
                .await?
                .into_iter()
                .map(|t| t.id)
                .collect(),
        ),
        _ => None,
    };
    let content_fields = req.url.is_some()
        || req.title.is_some()
        || req.description.is_some()
        || req.notes.is_some()
        || req.tags.is_some()
        || categories.is_some();

    let mut changes_out = None;
    if content_fields {
        let mut tx = pool.begin().await.map_err(db_err)?;
        // Concurrent saves of one bookmark queue here, and each writes only what it sent.
        let locked =
            sqlx::query_as::<_, Bookmark>("SELECT * FROM bookmarks WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?;

        if locked.is_vault {
            let key = *locked
                .vault_id
                .and_then(|id| keyring.get(id))
                .ok_or(Error::VaultLocked)?;
            let ct0 = locked
                .vault_ciphertext
                .clone()
                .ok_or(Error::msg("missing ciphertext"))?;
            let nonce0 = locked
                .vault_nonce
                .clone()
                .ok_or(Error::msg("missing nonce"))?;
            let mut payload =
                super::blocking::run_blocking(move || crypto::decrypt_payload(&key, &ct0, &nonce0))
                    .await?;
            changes_out = Some(bookmark_content_changes(&req, None));
            if let Some(url) = req.url.clone() {
                payload.url = url;
            }
            if let Some(title) = req.title.clone() {
                payload.title = title;
            }
            if let Some(description) = req.description.clone() {
                payload.description = description;
            }
            if let Some(notes) = req.notes.clone() {
                payload.notes = notes;
            }
            if let Some(tags) = req.tags.clone() {
                payload.tags = tags;
            }
            let (ct, nonce) =
                super::blocking::run_blocking(move || crypto::encrypt_payload(&key, &payload))
                    .await?;
            sqlx::query(
                r#"
                UPDATE bookmarks
                SET vault_ciphertext = $1, vault_nonce = $2, updated_at = now()
                WHERE id = $3
                "#,
            )
            .bind(ct)
            .bind(nonce)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        } else {
            changes_out = Some(bookmark_content_changes(&req, Some(locked.title.as_str())));
            sqlx::query(
                r#"
                UPDATE bookmarks
                SET url = COALESCE($1, url),
                    title = COALESCE($2, title),
                    description = CASE WHEN $3 THEN $4 ELSE description END,
                    notes = CASE WHEN $5 THEN $6 ELSE notes END,
                    updated_at = now(),
                    link_check_status = CASE WHEN $1 <> url THEN NULL ELSE link_check_status END,
                    link_check_status_code =
                        CASE WHEN $1 <> url THEN NULL ELSE link_check_status_code END,
                    link_check_error = CASE WHEN $1 <> url THEN NULL ELSE link_check_error END,
                    link_checked_at = CASE WHEN $1 <> url THEN NULL ELSE link_checked_at END,
                    link_check_fail_count =
                        CASE WHEN $1 <> url THEN 0 ELSE link_check_fail_count END,
                    preview_dismissed = CASE WHEN $1 <> url THEN FALSE ELSE preview_dismissed END,
                    preview_failed_at = CASE WHEN $1 <> url THEN NULL ELSE preview_failed_at END
                WHERE id = $7
                "#,
            )
            .bind(&req.url)
            .bind(&req.title)
            .bind(req.description.is_some())
            .bind(req.description.clone().flatten())
            .bind(req.notes.is_some())
            .bind(req.notes.clone().flatten())
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            if let Some(tag_ids) = &tag_ids {
                apply_tags(&mut tx, id, tag_ids).await?;
            }
        }
        if let Some(plan) = &categories {
            apply_categories(&mut tx, id, plan).await?;
        }
        tx.commit().await.map_err(db_err)?;
    }

    if let Some(is_private) = req.is_private {
        set_bookmark_private(pool, user_id, id, is_private).await?;
    }
    if flags {
        set_bookmark_flags(
            pool,
            user_id,
            id,
            req.is_favorite,
            req.is_unread,
            req.is_pinned,
        )
        .await?;
    }

    let b = get_bookmark_raw(pool, user_id, id).await?;
    let none = VaultKeyring::default();
    let ring = if b.is_vault { keyring } else { &none };
    if content_fields && !b.is_vault && b.owner_user_id != user_id {
        let title = if b.title.trim().is_empty() {
            b.url.as_str()
        } else {
            b.title.as_str()
        };
        super::notifications::notify_shared_edit(
            pool,
            user_id,
            b.owner_user_id,
            linkdean_core::NotificationSubjectType::Bookmark,
            b.id,
            title,
        )
        .await;
    }
    let view = bookmark_to_view(pool, b, ring, user_id)
        .await?
        .ok_or(Error::VaultLocked)?;
    if let Some((changes, change_keys)) = changes_out {
        record_bookmark_update_activity(pool, user_id, &view, &changes, &change_keys).await;
    }
    Ok(view)
}

/// Soft-delete: set `deleted_at`. Owner-only for vault; writers otherwise.
pub async fn delete_bookmark(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    if !bookmark_can_write(pool, user_id, id).await? {
        if bookmark_can_read(pool, user_id, id).await? {
            return Err(Error::Forbidden);
        }
        return Err(Error::NotFound);
    }
    let existing = get_bookmark_raw(pool, user_id, id).await?;
    if existing.is_vault && existing.owner_user_id != user_id {
        return Err(Error::Forbidden);
    }
    if existing.owner_user_id != user_id {
        // Invitees soft-delete is not supported — only owner can trash.
        return Err(Error::Forbidden);
    }
    if existing.deleted_at.is_some() {
        return Ok(());
    }
    let res = sqlx::query(
        r#"
        UPDATE bookmarks
        SET deleted_at = now(), updated_at = now()
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    let title = if existing.is_vault {
        "Vault bookmark".into()
    } else if existing.title.trim().is_empty() {
        existing.url.clone()
    } else {
        existing.title.clone()
    };
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Deleted,
        linkdean_core::ActivitySubjectType::Bookmark,
        id,
        &title,
    )
    .await;
    Ok(())
}

pub async fn restore_bookmark(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let title = soft_deleted_bookmark_title(pool, user_id, id).await?;
    let res = sqlx::query(
        r#"
        UPDATE bookmarks
        SET deleted_at = NULL, updated_at = now()
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Restored,
        linkdean_core::ActivitySubjectType::Bookmark,
        id,
        &title,
    )
    .await;
    Ok(())
}

/// Delete a bookmark, honouring the owner's choice for vault items.
pub async fn delete_bookmark_by_mode(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let live_vault_row: Option<(bool,)> = sqlx::query_as(
        r#"
        SELECT is_vault AND deleted_at IS NULL FROM bookmarks
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    delete_bookmark(pool, user_id, id).await?;
    if live_vault_row.is_some_and(|r| r.0)
        && super::users::get_user_settings(pool, user_id)
            .await?
            .vault_delete_mode
            == "permanent"
    {
        purge_bookmark(pool, user_id, id).await?;
    }
    Ok(())
}

pub async fn purge_bookmark(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let title = soft_deleted_bookmark_title(pool, user_id, id).await?;
    let res = sqlx::query(
        r#"
        DELETE FROM bookmarks
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    super::activity::record_activity_with(
        pool,
        user_id,
        linkdean_core::ActivityAction::Deleted,
        super::activity::ActivitySubject {
            subject_type: linkdean_core::ActivitySubjectType::Bookmark,
            subject_id: id,
            subject_title: &title,
        },
        Some(&format!("Permanently deleted bookmark: {title}")),
        serde_json::json!({ "purged": true }),
    )
    .await;
    Ok(())
}

async fn soft_deleted_bookmark_title(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<String> {
    #[derive(sqlx::FromRow)]
    struct Row {
        title: String,
        url: String,
        is_vault: bool,
    }
    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT title, url, is_vault FROM bookmarks
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
    if row.is_vault {
        Ok("Vault bookmark".into())
    } else if row.title.trim().is_empty() {
        Ok(row.url)
    } else {
        Ok(row.title)
    }
}

pub async fn update_bookmark_metadata(
    pool: &PgPool,
    id: Uuid,
    title: Option<&str>,
    description: Option<&str>,
    favicon_url: Option<&str>,
    preview_image_url: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            favicon_url = COALESCE($4, favicon_url),
            preview_image_url = COALESCE($5, preview_image_url),
            updated_at = now()
        WHERE id = $1 AND is_vault = FALSE
        "#,
    )
    .bind(id)
    .bind(title)
    .bind(description)
    .bind(favicon_url)
    .bind(preview_image_url)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Records a saved preview, or the owner removing it. A removed preview is not taken
/// again until the URL changes or the owner uploads one.
pub async fn set_bookmark_has_preview(pool: &PgPool, id: Uuid, has_preview: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET has_preview = $2, preview_dismissed = NOT $2, preview_failed_at = NULL,
            updated_at = now()
        WHERE id = $1 AND is_vault = FALSE
        "#,
    )
    .bind(id)
    .bind(has_preview)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// A screenshot failed. Backfills skip the bookmark for a week.
pub async fn record_preview_failure(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE bookmarks SET preview_failed_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Whether enrichment should take a screenshot: none saved, and the owner has not removed one.
pub async fn bookmark_wants_preview(pool: &PgPool, id: Uuid) -> Result<bool> {
    let wants: Option<bool> = sqlx::query_scalar(
        "SELECT NOT has_preview AND NOT preview_dismissed FROM bookmarks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(wants.unwrap_or(false))
}

/// Non-vault bookmarks that still need a local screenshot (owner_id, bookmark_id).
pub async fn list_bookmarks_needing_preview(pool: &PgPool) -> Result<Vec<(Uuid, Uuid)>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT owner_user_id, id FROM bookmarks
        WHERE is_vault = FALSE AND has_preview = FALSE AND deleted_at IS NULL AND url <> ''
          AND NOT preview_dismissed
          AND (preview_failed_at IS NULL OR preview_failed_at < now() - interval '7 days')
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows)
}

/// One user's bookmarks still missing a preview, such as those just imported.
pub async fn list_user_bookmarks_needing_preview(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, Uuid)>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT owner_user_id, id FROM bookmarks
        WHERE owner_user_id = $1
          AND is_vault = FALSE
          AND has_preview = FALSE
          AND deleted_at IS NULL
          AND url <> ''
          AND NOT preview_dismissed
          AND (preview_failed_at IS NULL OR preview_failed_at < now() - interval '7 days')
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows)
}

/// Claim a batch of bookmarks due for link checking (never-checked first, then stale).
/// Soft failures (`link_check_fail_count > 0` but not yet confirmed) recheck after
/// `soft_recheck_after`; healthy / confirmed rows use `recheck_after`.
pub async fn claim_bookmarks_for_link_check(
    pool: &PgPool,
    limit: i64,
    recheck_after: chrono::Duration,
    soft_recheck_after: chrono::Duration,
) -> Result<Vec<(Uuid, Uuid, String)>> {
    let ok_cutoff = chrono::Utc::now() - recheck_after;
    let soft_cutoff = chrono::Utc::now() - soft_recheck_after;
    let rows: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
        r#"
        SELECT owner_user_id, id, url FROM bookmarks
        WHERE is_vault = FALSE
          AND deleted_at IS NULL
          AND url <> ''
          AND (
            link_checked_at IS NULL
            OR (link_check_fail_count > 0 AND link_checked_at < $2)
            OR link_checked_at < $1
          )
        ORDER BY link_checked_at NULLS FIRST, created_at ASC
        LIMIT $3
        "#,
    )
    .bind(ok_cutoff)
    .bind(soft_cutoff)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows)
}

pub async fn record_link_check(
    pool: &PgPool,
    bookmark_id: Uuid,
    status: LinkCheckStatus,
    status_code: Option<i32>,
    error: Option<&str>,
) -> Result<()> {
    let row: Option<(Option<String>, i32)> = sqlx::query_as(
        r#"
        SELECT link_check_status, link_check_fail_count
        FROM bookmarks
        WHERE id = $1 AND is_vault = FALSE
        "#,
    )
    .bind(bookmark_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let Some((prev_status, prev_fails)) = row else {
        return Ok(());
    };

    let definitive_gone = matches!(status_code, Some(404) | Some(410));
    let (stored_status, fail_count): (Option<String>, i32) = if status.is_problem() {
        let fails = prev_fails.saturating_add(1);
        let store = if definitive_gone || fails >= 2 {
            Some(status.as_str().to_string())
        } else {
            // Soft fail: keep prior label (or still unchecked) until confirmed.
            prev_status
        };
        (store, fails)
    } else {
        (Some(status.as_str().to_string()), 0)
    };

    sqlx::query(
        r#"
        UPDATE bookmarks
        SET link_check_status = $2,
            link_check_status_code = $3,
            link_check_error = $4,
            link_check_fail_count = $5,
            link_checked_at = now()
        WHERE id = $1 AND is_vault = FALSE
        "#,
    )
    .bind(bookmark_id)
    .bind(stored_status.as_deref())
    .bind(status_code)
    .bind(error)
    .bind(fail_count)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Copy a readable non-vault bookmark into the current user's library (unsorted).
/// Skips notes; enrich worker can refetch favicon/OG for the new row.
pub async fn save_bookmark_copy(
    pool: &PgPool,
    user_id: Uuid,
    source_id: Uuid,
) -> Result<BookmarkView> {
    let raw = get_bookmark_raw(pool, user_id, source_id).await?;
    if raw.owner_user_id == user_id {
        return Err(Error::BadRequest(
            "bookmark is already in your library".into(),
        ));
    }
    if raw.is_vault {
        return Err(Error::BadRequest("cannot save vault bookmarks".into()));
    }
    let source = bookmark_to_view(pool, raw, &VaultKeyring::default(), user_id)
        .await?
        .ok_or(Error::NotFound)?;
    create_bookmark(
        pool,
        user_id,
        CreateBookmark {
            vault_id: None,
            url: source.url,
            title: Some(source.title),
            description: source.description,
            notes: None,
            category_id: None,
            category_ids: Vec::new(),
            tags: source.tags,
            is_favorite: false,
            is_unread: false,
            is_pinned: false,
        },
        &VaultKeyring::default(),
    )
    .await
}

/// Owner-only list of bookmarks currently flagged broken / unreachable.
pub async fn list_broken_bookmarks(pool: &PgPool, user_id: Uuid) -> Result<Vec<BookmarkView>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT * FROM bookmarks
        WHERE owner_user_id = $1
          AND is_vault = FALSE
          AND deleted_at IS NULL
          AND link_check_status IN ('broken', 'unreachable')
        ORDER BY link_checked_at DESC NULLS LAST, updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let none = VaultKeyring::default();
    bookmarks_to_views(pool, rows, user_id, |_| &none).await
}

pub async fn find_duplicate_urls(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT url FROM bookmarks
        WHERE owner_user_id = $1 AND is_vault = FALSE AND deleted_at IS NULL AND url <> ''
        GROUP BY url HAVING COUNT(*) > 1
        ORDER BY url
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(|(u,)| u).collect())
}

/// Duplicate URL groups with full bookmark views (owner, non-vault only).
pub async fn find_duplicate_groups(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(String, Vec<BookmarkView>)>> {
    let urls = find_duplicate_urls(pool, user_id).await?;
    let mut groups = Vec::with_capacity(urls.len());
    for url in urls {
        let rows = sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT * FROM bookmarks
            WHERE owner_user_id = $1 AND is_vault = FALSE AND deleted_at IS NULL AND url = $2
            ORDER BY updated_at DESC, created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(&url)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let none = VaultKeyring::default();
        let bookmarks = bookmarks_to_views(pool, rows, user_id, |_| &none).await?;
        if bookmarks.len() > 1 {
            groups.push((url, bookmarks));
        }
    }
    Ok(groups)
}
