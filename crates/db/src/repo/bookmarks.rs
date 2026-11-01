use linkdean_core::crypto;
use linkdean_core::{
    Bookmark, BookmarkView, Category, CategoryRef, CreateBookmark, Error, LinkCheckStatus, Result,
    UpdateBookmark, VaultPayload,
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
            link_check_status: None,
            link_check_status_code: None,
            link_check_error: None,
            link_checked_at: None,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }));
    }
    let tags = tags_for_bookmark(pool, b.id).await?;
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
            owned.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        }
        owned
    };

    let mut out = Vec::new();
    for b in rows {
        let key = if b.is_vault && b.owner_user_id == user_id {
            vault_key
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
        set_bookmark_flags(
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
    let view = bookmark_to_view(pool, bookmark, vault_key, user_id)
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
}

fn update_has_flags(req: &UpdateBookmark) -> bool {
    req.is_favorite.is_some() || req.is_unread.is_some() || req.is_pinned.is_some()
}

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
        match before_title.map(str::trim).filter(|s| !s.is_empty()) {
            Some(prev) if prev != title.trim() => {
                changes.push(format!("renamed “{prev}” → “{}”", title.trim()));
            }
            _ => changes.push(format!("title set to “{}”", title.trim())),
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
        let (changes, change_keys) = bookmark_content_changes(&req, Some(payload.title.as_str()));
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
        let view = bookmark_to_view(pool, b, vault_key, user_id)
            .await?
            .ok_or(Error::VaultLocked)?;
        record_bookmark_update_activity(pool, user_id, &view, &changes, &change_keys).await;
        return Ok(view);
    }

    let (changes, change_keys) = bookmark_content_changes(&req, Some(existing.title.as_str()));
    let url_changed = req.url.as_ref().is_some_and(|u| u != &existing.url);
    let url = req.url.unwrap_or(existing.url);
    let title = req.title.unwrap_or(existing.title);
    let description = req.description.or(existing.description);
    let notes = req.notes.or(existing.notes);
    if url_changed {
        sqlx::query(
            r#"
            UPDATE bookmarks
            SET url=$1, title=$2, description=$3, notes=$4, updated_at=now(),
                link_check_status = NULL,
                link_check_status_code = NULL,
                link_check_error = NULL,
                link_checked_at = NULL,
                link_check_fail_count = 0
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
    } else {
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
    }
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
    let view = bookmark_to_view(pool, b, None, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    record_bookmark_update_activity(pool, user_id, &view, &changes, &change_keys).await;
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
            link_checked_at = now(),
            updated_at = now()
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
    let source = bookmark_to_view(pool, raw, None, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    create_bookmark(
        pool,
        user_id,
        CreateBookmark {
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
        None,
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

    let mut out = Vec::with_capacity(rows.len());
    for b in rows {
        if let Some(view) = bookmark_to_view(pool, b, None, user_id).await? {
            out.push(view);
        }
    }
    Ok(out)
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
