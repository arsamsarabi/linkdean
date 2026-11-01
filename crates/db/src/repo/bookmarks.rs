use linkdean_core::crypto;
use linkdean_core::{
    Bookmark, BookmarkView, Category, CategoryRef, CreateBookmark, Error, Result, UpdateBookmark,
    VaultPayload,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use super::categories::{get_category_by_id, list_owned_categories};
use super::shares::{
    bookmark_can_read, bookmark_can_write, bookmark_shared_with, require_category_read,
    require_category_write,
};
use super::tags::ensure_tags;

async fn set_bookmark_tags(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    tags: &[String],
) -> Result<Vec<String>> {
    sqlx::query("DELETE FROM bookmark_tags WHERE bookmark_id = $1")
        .bind(bookmark_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let ensured = ensure_tags(pool, user_id, tags).await?;
    for t in &ensured {
        sqlx::query(
            "INSERT INTO bookmark_tags (bookmark_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(bookmark_id)
        .bind(t.id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }
    Ok(ensured.into_iter().map(|t| t.name).collect())
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

async fn tags_for_bookmark(pool: &PgPool, bookmark_id: Uuid) -> Result<Vec<String>> {
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

async fn categories_for_bookmark(
    pool: &PgPool,
    owner_user_id: Uuid,
    bookmark_id: Uuid,
) -> Result<Vec<CategoryRef>> {
    let linked: Vec<Uuid> = sqlx::query_as(
        r#"
        SELECT category_id FROM bookmark_categories
        WHERE bookmark_id = $1
        ORDER BY category_id
        "#,
    )
    .bind(bookmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    if linked.is_empty() {
        return Ok(Vec::new());
    }

    let all = list_owned_categories(pool, owner_user_id, true, true).await?;
    let by_id: HashMap<Uuid, Category> = all.into_iter().map(|c| (c.id, c)).collect();
    let mut out = Vec::new();
    for id in linked {
        if let Some(cat) = by_id.get(&id) {
            out.push(CategoryRef {
                id: cat.id,
                name: cat.name.clone(),
                path: category_path(cat, &by_id),
            });
        } else if let Ok(cat) = get_category_by_id(pool, id).await {
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

async fn set_bookmark_categories(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    category_ids: &[Uuid],
) -> Result<Option<Uuid>> {
    let mut unique = Vec::new();
    for id in category_ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }

    let mut any_vault = false;
    let mut any_non_vault = false;
    for id in &unique {
        let (cat, _) = require_category_write(pool, user_id, *id).await?;
        if cat.is_vault {
            any_vault = true;
        } else {
            any_non_vault = true;
        }
    }
    if any_vault && any_non_vault {
        return Err(Error::BadRequest(
            "cannot mix vault and non-vault categories on one bookmark".into(),
        ));
    }

    sqlx::query("DELETE FROM bookmark_categories WHERE bookmark_id = $1")
        .bind(bookmark_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    for id in &unique {
        sqlx::query(
            "INSERT INTO bookmark_categories (bookmark_id, category_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(bookmark_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }

    let primary = unique.first().copied();
    sqlx::query("UPDATE bookmarks SET category_id = $2 WHERE id = $1")
        .bind(bookmark_id)
        .bind(primary)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(primary)
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
    vault_key: Option<&[u8; 32]>,
    viewer_id: Uuid,
) -> Result<Option<BookmarkView>> {
    let categories = categories_for_bookmark(pool, b.owner_user_id, b.id).await?;
    let category_id = categories.first().map(|c| c.id).or(b.category_id);
    let can_write = if b.owner_user_id == viewer_id {
        true
    } else {
        bookmark_can_write(pool, viewer_id, b.id).await?
    };
    let shared_with = if b.owner_user_id == viewer_id {
        bookmark_shared_with(pool, b.id).await?
    } else {
        Vec::new()
    };
    let shared_out = !shared_with.is_empty();
    let owner_display_name =
        super::users::owner_display_name_for_viewer(pool, b.owner_user_id, viewer_id).await?;
    let flags = flags_for_viewer(pool, viewer_id, b.id).await?;

    if b.is_vault {
        let key = match vault_key {
            Some(k) => k,
            None => return Ok(None),
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
            is_vault: true,
            tags: payload.tags,
            can_write,
            shared_out,
            shared_with: shared_with.clone(),
            owner_display_name: owner_display_name.clone(),
            is_favorite: flags.is_favorite,
            is_unread: flags.is_unread,
            is_pinned: flags.is_pinned,
            deleted_at: b.deleted_at,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }));
    }
    let tags = tags_for_bookmark(pool, b.id).await?;
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
        is_vault: false,
        tags,
        can_write,
        shared_out,
        shared_with,
        owner_display_name,
        is_favorite: flags.is_favorite,
        is_unread: flags.is_unread,
        is_pinned: flags.is_pinned,
        deleted_at: b.deleted_at,
        created_at: b.created_at,
        updated_at: b.updated_at,
    }))
}

pub async fn list_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    category_id: Option<Uuid>,
    include_vault: bool,
    vault_key: Option<&[u8; 32]>,
    include_deleted: bool,
) -> Result<Vec<BookmarkView>> {
    use super::shares::list_shared_categories;

    let include_vault = include_vault && vault_key.is_some();
    let rows = if let Some(cid) = category_id {
        require_category_read(pool, user_id, cid).await?;
        sqlx::query_as::<_, Bookmark>(
            r#"
            SELECT b.* FROM bookmarks b
            WHERE ($2 OR b.is_vault = FALSE)
              AND ($3 OR b.deleted_at IS NULL)
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
            owned.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        }
        owned
    };

    let mut out = Vec::new();
    for b in rows {
        let key = if b.is_vault && b.owner_user_id == user_id {
            vault_key
        } else if b.is_vault {
            None
        } else {
            None
        };
        if let Some(v) = bookmark_to_view(pool, b, key, user_id).await? {
            out.push(v);
        }
    }
    Ok(out)
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

pub async fn create_bookmark(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateBookmark,
    vault_key: Option<&[u8; 32]>,
) -> Result<BookmarkView> {
    let cat_ids = resolve_category_ids(&req.category_ids, req.category_id);
    let mut is_vault = false;
    for cid in &cat_ids {
        let (cat, access) = require_category_write(pool, user_id, *cid).await?;
        if cat.is_vault {
            if !access.is_owner() {
                return Err(Error::Forbidden);
            }
            is_vault = true;
        }
    }

    let title = req.title.clone().unwrap_or_else(|| req.url.clone());
    let primary = cat_ids.first().copied();

    let bookmark = if is_vault {
        let key = vault_key.ok_or(Error::VaultLocked)?;
        let payload = VaultPayload {
            url: req.url.clone(),
            title: title.clone(),
            description: req.description.clone(),
            notes: req.notes.clone(),
            tags: req.tags.clone(),
        };
        let key = *key;
        let (ct, nonce) =
            super::blocking::run_blocking(move || crypto::encrypt_payload(&key, &payload)).await?;
        sqlx::query_as::<_, Bookmark>(
            r#"
            INSERT INTO bookmarks (
                owner_user_id, category_id, url, title, description, notes,
                is_vault, vault_ciphertext, vault_nonce
            ) VALUES ($1,$2,'','','',NULL,TRUE,$3,$4)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(primary)
        .bind(ct)
        .bind(nonce)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
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
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        set_bookmark_tags(pool, user_id, b.id, &req.tags).await?;
        b
    };

    set_bookmark_categories(pool, user_id, bookmark.id, &cat_ids).await?;
    if req.is_favorite || req.is_unread || req.is_pinned {
        let _ = set_bookmark_flags(
            pool,
            user_id,
            bookmark.id,
            Some(req.is_favorite),
            Some(req.is_unread),
            Some(req.is_pinned),
        )
        .await?;
    }
    let bookmark = get_bookmark_raw(pool, user_id, bookmark.id).await?;

    bookmark_to_view(pool, bookmark, vault_key, user_id)
        .await?
        .ok_or(Error::VaultLocked)
}

fn update_has_content(req: &UpdateBookmark) -> bool {
    req.url.is_some()
        || req.title.is_some()
        || req.description.is_some()
        || req.notes.is_some()
        || req.category_id.is_some()
        || req.category_ids.is_some()
        || req.tags.is_some()
}

fn update_has_flags(req: &UpdateBookmark) -> bool {
    req.is_favorite.is_some() || req.is_unread.is_some() || req.is_pinned.is_some()
}

pub async fn update_bookmark(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateBookmark,
    vault_key: Option<&[u8; 32]>,
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

    if flags {
        let _ = set_bookmark_flags(
            pool,
            user_id,
            id,
            req.is_favorite,
            req.is_unread,
            req.is_pinned,
        )
        .await?;
    }
    if !content {
        let existing = get_bookmark_raw(pool, user_id, id).await?;
        let key = if existing.is_vault { vault_key } else { None };
        return bookmark_to_view(pool, existing, key, user_id)
            .await?
            .ok_or(Error::VaultLocked);
    }

    let existing = get_bookmark_raw(pool, user_id, id).await?;
    // Vault mutations only by bookmark owner with their key.
    if existing.is_vault && existing.owner_user_id != user_id {
        return Err(Error::Forbidden);
    }
    if existing.is_vault {
        let key = *vault_key.ok_or(Error::VaultLocked)?;
        let ct0 = existing
            .vault_ciphertext
            .clone()
            .ok_or(Error::msg("missing ciphertext"))?;
        let nonce0 = existing
            .vault_nonce
            .clone()
            .ok_or(Error::msg("missing nonce"))?;
        let mut payload =
            super::blocking::run_blocking(move || crypto::decrypt_payload(&key, &ct0, &nonce0))
                .await?;
        if let Some(url) = req.url {
            payload.url = url;
        }
        if let Some(title) = req.title {
            payload.title = title;
        }
        if let Some(description) = req.description {
            payload.description = Some(description);
        }
        if let Some(notes) = req.notes {
            payload.notes = Some(notes);
        }
        if let Some(tags) = req.tags {
            payload.tags = tags;
        }
        let (ct, nonce) =
            super::blocking::run_blocking(move || crypto::encrypt_payload(&key, &payload)).await?;
        sqlx::query_as::<_, Bookmark>(
            r#"
            UPDATE bookmarks
            SET vault_ciphertext = $1, vault_nonce = $2, updated_at = now()
            WHERE id = $3 AND owner_user_id = $4
            RETURNING *
            "#,
        )
        .bind(ct)
        .bind(nonce)
        .bind(id)
        .bind(existing.owner_user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if let Some(ids) = req.category_ids {
            set_bookmark_categories(pool, user_id, id, &ids).await?;
        } else if let Some(c) = req.category_id {
            set_bookmark_categories(pool, user_id, id, &c.into_iter().collect::<Vec<_>>()).await?;
        }
        let b = get_bookmark_raw(pool, user_id, id).await?;
        return bookmark_to_view(pool, b, vault_key, user_id)
            .await?
            .ok_or(Error::VaultLocked);
    }

    let url = req.url.unwrap_or(existing.url);
    let title = req.title.unwrap_or(existing.title);
    let description = req.description.or(existing.description);
    let notes = req.notes.or(existing.notes);
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET url=$1, title=$2, description=$3, notes=$4, updated_at=now()
        WHERE id=$5
        "#,
    )
    .bind(&url)
    .bind(&title)
    .bind(&description)
    .bind(&notes)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if let Some(tags) = req.tags {
        // Tags stay in the actor's tag namespace.
        set_bookmark_tags(pool, user_id, id, &tags).await?;
    }
    if let Some(ids) = req.category_ids {
        set_bookmark_categories(pool, user_id, id, &ids).await?;
    } else if let Some(c) = req.category_id {
        set_bookmark_categories(pool, user_id, id, &c.into_iter().collect::<Vec<_>>()).await?;
    }
    let b = get_bookmark_raw(pool, user_id, id).await?;
    if b.owner_user_id != user_id {
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
    bookmark_to_view(pool, b, None, user_id)
        .await?
        .ok_or(Error::NotFound)
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
    Ok(())
}

pub async fn restore_bookmark(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
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
    Ok(())
}

pub async fn purge_bookmark(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
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
    Ok(())
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

pub async fn set_bookmark_has_preview(pool: &PgPool, id: Uuid, has_preview: bool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE bookmarks
        SET has_preview = $2, updated_at = now()
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

/// Non-vault bookmarks that still need a local screenshot (owner_id, bookmark_id).
pub async fn list_bookmarks_needing_preview(pool: &PgPool) -> Result<Vec<(Uuid, Uuid)>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT owner_user_id, id FROM bookmarks
        WHERE is_vault = FALSE AND has_preview = FALSE AND url <> ''
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows)
}

pub async fn find_duplicate_urls(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT url FROM bookmarks
        WHERE owner_user_id = $1 AND is_vault = FALSE AND url <> ''
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
            WHERE owner_user_id = $1 AND is_vault = FALSE AND url = $2
            ORDER BY updated_at DESC, created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(&url)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let mut bookmarks = Vec::with_capacity(rows.len());
        for b in rows {
            if let Some(view) = bookmark_to_view(pool, b, None, user_id).await? {
                bookmarks.push(view);
            }
        }
        if bookmarks.len() > 1 {
            groups.push((url, bookmarks));
        }
    }
    Ok(groups)
}
