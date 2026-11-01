use linkdean_core::crypto;
use linkdean_core::{
    BookmarkView, Category, CategoryAccess, CategoryPublicLinkView, CategoryView, CreateBookmark,
    CreateCategory, Error, ForkCategory, PublicBookmarkView, PublicListView, Result,
    UpdateCategory,
};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::bookmarks::{bookmark_to_view, create_bookmark, get_bookmark_raw, tags_for_bookmark};
use super::shares::{
    bookmark_can_read, category_access, list_shared_categories, owned_shared_out_invitees,
    require_category_owner, require_category_read, require_category_write,
};
use super::users::{find_user_by_id, user_share_label};

const PUBLIC_SLUG_PREFIX: &str = "ldl_";

fn public_list_path(slug: &str) -> String {
    format!("/l/{slug}")
}

fn new_public_slug() -> String {
    let secret = hex::encode(crypto::random_bytes(16));
    format!("{PUBLIC_SLUG_PREFIX}{secret}")
}

fn to_public_link_view(slug: Option<&str>) -> CategoryPublicLinkView {
    match slug {
        Some(s) if !s.is_empty() => CategoryPublicLinkView {
            enabled: true,
            slug: Some(s.to_string()),
            path: Some(public_list_path(s)),
        },
        _ => CategoryPublicLinkView {
            enabled: false,
            slug: None,
            path: None,
        },
    }
}

/// Allowlist of category icon keys. Unknown / empty → `None` (default folder).
fn normalize_category_icon(icon: Option<String>) -> Option<String> {
    const ALLOWED: &[&str] = &[
        "bookmark",
        "star",
        "heart",
        "code",
        "book",
        "briefcase",
        "home",
        "music",
        "photo",
        "world",
        "gamepad",
        "cart",
        "school",
        "flask",
        "plane",
        "car",
        "pizza",
        "coffee",
        "movie",
        "news",
        "tools",
        "lock",
    ];
    let key = icon?.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }
    ALLOWED
        .iter()
        .find(|&&a| a == key)
        .map(|s| (*s).to_string())
}

pub async fn get_category_by_id(pool: &PgPool, id: Uuid) -> Result<Category> {
    sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

pub async fn list_categories(
    pool: &PgPool,
    user_id: Uuid,
    include_hidden: bool,
    include_vault: bool,
    include_deleted: bool,
) -> Result<Vec<CategoryView>> {
    let owned = sqlx::query_as::<_, Category>(
        r#"
        SELECT * FROM categories
        WHERE owner_user_id = $1
          AND ($2 OR is_hidden = FALSE)
          AND ($3 OR is_vault = FALSE)
          AND ($4 OR deleted_at IS NULL)
        ORDER BY position, name
        "#,
    )
    .bind(user_id)
    .bind(include_hidden)
    .bind(include_vault)
    .bind(include_deleted)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let shared_out = owned_shared_out_invitees(pool, user_id).await?;

    let mut by_id: HashMap<Uuid, CategoryView> = owned
        .into_iter()
        .map(|c| {
            let id = c.id;
            let mut view = CategoryView::from_category(c, CategoryAccess::Owner);
            if let Some(labels) = shared_out.get(&id) {
                view.shared_out = true;
                view.shared_with = labels.clone();
            }
            (id, view)
        })
        .collect();

    let shared = list_shared_categories(pool, user_id, include_hidden).await?;
    for (cat, access) in shared {
        by_id
            .entry(cat.id)
            .or_insert_with(|| CategoryView::from_category(cat, access));
    }

    let mut out: Vec<CategoryView> = by_id.into_values().collect();
    for view in &mut out {
        if !view.access.is_owner() {
            view.owner_display_name =
                super::users::owner_display_name_for_viewer(pool, view.owner_user_id, user_id)
                    .await?;
        }
    }
    out.sort_by(|a, b| {
        a.position
            .cmp(&b.position)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

/// Owned categories only (no shares). Used when resolving paths for a bookmark owner.
pub async fn list_owned_categories(
    pool: &PgPool,
    user_id: Uuid,
    include_hidden: bool,
    include_vault: bool,
) -> Result<Vec<Category>> {
    sqlx::query_as::<_, Category>(
        r#"
        SELECT * FROM categories
        WHERE owner_user_id = $1
          AND ($2 OR is_hidden = FALSE)
          AND ($3 OR is_vault = FALSE)
          AND deleted_at IS NULL
        ORDER BY position, name
        "#,
    )
    .bind(user_id)
    .bind(include_hidden)
    .bind(include_vault)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<CategoryView> {
    let access = category_access(pool, user_id, id).await?;
    let cat = get_category_by_id(pool, id).await?;
    let mut view = CategoryView::from_category(cat, access);
    if view.access.is_owner() {
        if let Some(labels) = owned_shared_out_invitees(pool, user_id).await?.remove(&id) {
            view.shared_out = true;
            view.shared_with = labels;
        }
    } else {
        view.owner_display_name =
            super::users::owner_display_name_for_viewer(pool, view.owner_user_id, user_id).await?;
    }
    Ok(view)
}

pub async fn create_category(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateCategory,
) -> Result<CategoryView> {
    let owner_user_id = if let Some(pid) = req.parent_id {
        let (parent, access) = require_category_write(pool, user_id, pid).await?;
        if parent.is_vault && !req.is_vault {
            return Err(Error::BadRequest(
                "non-vault category cannot nest under vault".into(),
            ));
        }
        if req.is_vault && !access.is_owner() {
            return Err(Error::Forbidden);
        }
        // Child under shared folder stays under folder owner's tree.
        parent.owner_user_id
    } else {
        if req.is_vault {
            // vault roots only for self
        }
        user_id
    };

    if owner_user_id != user_id && req.is_vault {
        return Err(Error::Forbidden);
    }

    let icon = normalize_category_icon(req.icon);
    let cat = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (owner_user_id, parent_id, name, is_hidden, is_vault, icon)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(owner_user_id)
    .bind(req.parent_id)
    .bind(req.name)
    .bind(req.is_hidden)
    .bind(req.is_vault)
    .bind(icon)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let access = category_access(pool, user_id, cat.id).await?;
    let view = CategoryView::from_category(cat, access);
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Created,
        linkdean_core::ActivitySubjectType::Category,
        view.id,
        &view.name,
    )
    .await;
    Ok(view)
}

pub async fn update_category(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateCategory,
) -> Result<CategoryView> {
    let (mut cat, access) = require_category_write(pool, user_id, id).await?;
    let before_name = cat.name.clone();
    let before_parent = cat.parent_id;
    let before_hidden = cat.is_hidden;
    let before_position = cat.position;
    let before_icon = cat.icon.clone();

    if let Some(name) = req.name {
        cat.name = name;
    }
    if let Some(parent) = req.parent_id {
        if let Some(pid) = parent {
            // Moving: need write on new parent; owner-only if leaving owner's tree.
            let (new_parent, _) = require_category_write(pool, user_id, pid).await?;
            if new_parent.owner_user_id != cat.owner_user_id && !access.is_owner() {
                return Err(Error::Forbidden);
            }
            if new_parent.is_vault && !cat.is_vault {
                return Err(Error::BadRequest(
                    "non-vault category cannot nest under vault".into(),
                ));
            }
        } else if !access.is_owner() {
            // Non-owners cannot promote shared child to root under themselves.
            return Err(Error::Forbidden);
        }
        cat.parent_id = parent;
    }
    if let Some(h) = req.is_hidden {
        if !access.is_owner() {
            return Err(Error::Forbidden);
        }
        cat.is_hidden = h;
    }
    if let Some(p) = req.position {
        cat.position = p;
    }
    if let Some(icon) = req.icon {
        cat.icon = normalize_category_icon(icon);
    }

    let mut changes: Vec<String> = Vec::new();
    let mut change_keys: Vec<&'static str> = Vec::new();
    if cat.name != before_name {
        changes.push(format!("renamed “{before_name}” → “{}”", cat.name));
        change_keys.push("name");
    }
    if cat.parent_id != before_parent {
        match (before_parent, cat.parent_id) {
            (_, None) => changes.push("moved to root".into()),
            (None, Some(_)) => changes.push("moved under another list".into()),
            _ => changes.push("moved".into()),
        }
        change_keys.push("parent_id");
    }
    if cat.is_hidden != before_hidden {
        changes.push(if cat.is_hidden {
            "hidden".into()
        } else {
            "unhidden".into()
        });
        change_keys.push("is_hidden");
    }
    if cat.position != before_position {
        changes.push("reordered".into());
        change_keys.push("position");
    }
    if cat.icon != before_icon {
        match cat.icon.as_deref() {
            Some(icon) => changes.push(format!("icon set to {icon}")),
            None => changes.push("icon cleared".into()),
        }
        change_keys.push("icon");
    }

    let updated = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET name = $1, parent_id = $2, is_hidden = $3, position = $4, icon = $5, updated_at = now()
        WHERE id = $6
        RETURNING *
        "#,
    )
    .bind(&cat.name)
    .bind(cat.parent_id)
    .bind(cat.is_hidden)
    .bind(cat.position)
    .bind(&cat.icon)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if !access.is_owner() {
        super::notifications::notify_shared_edit(
            pool,
            user_id,
            updated.owner_user_id,
            linkdean_core::NotificationSubjectType::Category,
            updated.id,
            &updated.name,
        )
        .await;
    }

    let access = category_access(pool, user_id, updated.id).await?;
    let view = CategoryView::from_category(updated, access);
    if !changes.is_empty() {
        let summary = super::activity::update_summary(
            linkdean_core::ActivitySubjectType::Category,
            &view.name,
            &changes,
        );
        super::activity::record_activity_with(
            pool,
            user_id,
            linkdean_core::ActivityAction::Updated,
            linkdean_core::ActivitySubjectType::Category,
            view.id,
            &view.name,
            Some(&summary),
            serde_json::json!({ "changes": change_keys }),
        )
        .await;
    }
    Ok(view)
}

/// Soft-delete category subtree (descendants included). Owner-only.
pub async fn delete_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    require_category_owner(pool, user_id, id).await?;
    let cat = get_category_by_id(pool, id).await?;
    if cat.deleted_at.is_some() {
        return Ok(());
    }
    let title = cat.name.clone();
    // Collect subtree ids (owned).
    let owned = list_owned_categories(pool, user_id, true, true).await?;
    let mut ids = vec![id];
    let mut changed = true;
    while changed {
        changed = false;
        for c in &owned {
            if let Some(pid) = c.parent_id {
                if ids.contains(&pid) && !ids.contains(&c.id) && c.deleted_at.is_none() {
                    ids.push(c.id);
                    changed = true;
                }
            }
        }
    }
    let res = sqlx::query(
        r#"
        UPDATE categories
        SET deleted_at = now(), updated_at = now()
        WHERE owner_user_id = $1 AND id = ANY($2) AND deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .bind(&ids)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Deleted,
        linkdean_core::ActivitySubjectType::Category,
        id,
        &title,
    )
    .await;
    Ok(())
}

pub async fn restore_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    require_category_owner(pool, user_id, id).await?;
    // Restore this node + descendants that share the same soft-delete wave
    // (deleted_at not null under this root). Simpler: restore subtree by parent walk among deleted.
    let owned = sqlx::query_as::<_, Category>(
        r#"
        SELECT * FROM categories
        WHERE owner_user_id = $1 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let title = owned
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.clone())
        .ok_or(Error::NotFound)?;
    let mut ids = vec![id];
    let mut changed = true;
    while changed {
        changed = false;
        for c in &owned {
            if let Some(pid) = c.parent_id {
                if ids.contains(&pid) && !ids.contains(&c.id) {
                    ids.push(c.id);
                    changed = true;
                }
            }
        }
    }
    sqlx::query(
        r#"
        UPDATE categories
        SET deleted_at = NULL, updated_at = now()
        WHERE owner_user_id = $1 AND id = ANY($2)
        "#,
    )
    .bind(user_id)
    .bind(&ids)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Restored,
        linkdean_core::ActivitySubjectType::Category,
        id,
        &title,
    )
    .await;
    Ok(())
}

pub async fn purge_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    require_category_owner(pool, user_id, id).await?;
    let title = sqlx::query_scalar::<_, String>(
        r#"
        SELECT name FROM categories
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
    let res = sqlx::query(
        r#"
        DELETE FROM categories
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
        linkdean_core::ActivitySubjectType::Category,
        id,
        &title,
        Some(&format!("Permanently deleted list: {title}")),
        serde_json::json!({ "purged": true }),
    )
    .await;
    Ok(())
}

/// Fork a readable non-vault category tree into a new owned root + bookmark copies.
/// Returns `(new_root, copied_bookmarks)`.
pub async fn fork_category(
    pool: &PgPool,
    user_id: Uuid,
    source_id: Uuid,
    req: ForkCategory,
) -> Result<(CategoryView, Vec<BookmarkView>)> {
    let (root, _) = require_category_read(pool, user_id, source_id).await?;
    if root.is_vault {
        return Err(Error::BadRequest("cannot fork vault categories".into()));
    }

    let subtree: Vec<Category> = sqlx::query_as(
        r#"
        WITH RECURSIVE tree AS (
            SELECT * FROM categories WHERE id = $1 AND is_vault = FALSE
            UNION ALL
            SELECT c.* FROM categories c
            JOIN tree ON c.parent_id = tree.id
            WHERE c.is_vault = FALSE
        )
        SELECT * FROM tree
        "#,
    )
    .bind(source_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    if subtree.is_empty() {
        return Err(Error::NotFound);
    }

    let new_root_name = req
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("{} (copy)", root.name));

    let mut id_map: HashMap<Uuid, Uuid> = HashMap::new();
    let mut remaining = subtree;
    let mut guard = 0;
    while !remaining.is_empty() {
        guard += 1;
        if guard > 10_000 {
            return Err(Error::msg("category fork tree too deep or cyclic"));
        }
        let mut next = Vec::new();
        let mut progressed = false;
        for cat in remaining {
            let new_parent = if cat.id == source_id {
                None
            } else {
                match cat.parent_id.and_then(|pid| id_map.get(&pid).copied()) {
                    Some(mapped) => Some(mapped),
                    None => {
                        next.push(cat);
                        continue;
                    }
                }
            };
            let name = if cat.id == source_id {
                new_root_name.clone()
            } else {
                cat.name.clone()
            };
            let created = create_category(
                pool,
                user_id,
                CreateCategory {
                    name,
                    parent_id: new_parent,
                    is_hidden: cat.is_hidden,
                    is_vault: false,
                    icon: cat.icon.clone(),
                },
            )
            .await?;
            id_map.insert(cat.id, created.id);
            progressed = true;
        }
        if !progressed {
            return Err(Error::msg("category fork could not resolve parent order"));
        }
        remaining = next;
    }

    let old_ids: Vec<Uuid> = id_map.keys().copied().collect();
    let mut bookmark_ids: HashSet<Uuid> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT bookmark_id FROM bookmark_categories
        WHERE category_id = ANY($1)
        "#,
    )
    .bind(&old_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .into_iter()
    .collect();

    let legacy_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM bookmarks
        WHERE is_vault = FALSE AND category_id = ANY($1)
        "#,
    )
    .bind(&old_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    bookmark_ids.extend(legacy_ids);

    let mut copied = Vec::new();
    for bid in bookmark_ids {
        if !bookmark_can_read(pool, user_id, bid).await? {
            continue;
        }
        let raw = match get_bookmark_raw(pool, user_id, bid).await {
            Ok(b) => b,
            Err(_) => continue,
        };
        if raw.is_vault {
            continue;
        }
        let Some(view) = bookmark_to_view(pool, raw, None, user_id).await? else {
            continue;
        };
        let mut mapped_cats: Vec<Uuid> = view
            .categories
            .iter()
            .filter_map(|c| id_map.get(&c.id).copied())
            .collect();
        if mapped_cats.is_empty() {
            if let Some(new_root) = id_map.get(&source_id) {
                mapped_cats.push(*new_root);
            } else {
                continue;
            }
        }
        let created = create_bookmark(
            pool,
            user_id,
            CreateBookmark {
                url: view.url,
                title: Some(view.title),
                description: view.description,
                notes: view.notes,
                category_id: mapped_cats.first().copied(),
                category_ids: mapped_cats,
                tags: view.tags,
                ..Default::default()
            },
            None,
        )
        .await?;
        copied.push(created);
    }

    let new_root_id = *id_map
        .get(&source_id)
        .ok_or_else(|| Error::msg("fork root missing"))?;
    let view = get_category(pool, user_id, new_root_id).await?;
    Ok((view, copied))
}

/// Owner: current public-link state for a category.
pub async fn get_category_public_link(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
) -> Result<CategoryPublicLinkView> {
    let cat = require_category_owner(pool, owner_user_id, category_id).await?;
    Ok(to_public_link_view(cat.public_slug.as_deref()))
}

/// Owner: enable public link (idempotent if already enabled).
pub async fn enable_category_public_link(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
) -> Result<CategoryPublicLinkView> {
    let cat = require_category_owner(pool, owner_user_id, category_id).await?;
    if cat.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    if cat.is_vault {
        return Err(Error::BadRequest(
            "vault categories cannot have a public link".into(),
        ));
    }
    if let Some(slug) = cat.public_slug.as_deref().filter(|s| !s.is_empty()) {
        return Ok(to_public_link_view(Some(slug)));
    }

    // Retry once on rare slug collision.
    for _ in 0..2 {
        let slug = new_public_slug();
        let updated = sqlx::query_as::<_, Category>(
            r#"
            UPDATE categories
            SET public_slug = $1, updated_at = now()
            WHERE id = $2
              AND owner_user_id = $3
              AND is_vault = FALSE
              AND deleted_at IS NULL
              AND public_slug IS NULL
            RETURNING *
            "#,
        )
        .bind(&slug)
        .bind(category_id)
        .bind(owner_user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                Error::Conflict("public slug collision".into())
            } else {
                Error::msg(e.to_string())
            }
        })?;

        if let Some(row) = updated {
            return Ok(to_public_link_view(row.public_slug.as_deref()));
        }

        // Lost race: another request enabled first — return current.
        let again = get_category_by_id(pool, category_id).await?;
        if let Some(s) = again.public_slug.as_deref().filter(|s| !s.is_empty()) {
            return Ok(to_public_link_view(Some(s)));
        }
    }

    Err(Error::msg("failed to enable public link"))
}

/// Owner: rotate slug (invalidates old URL). No-op enable if disabled.
pub async fn rotate_category_public_link(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
) -> Result<CategoryPublicLinkView> {
    let cat = require_category_owner(pool, owner_user_id, category_id).await?;
    if cat.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    if cat.is_vault {
        return Err(Error::BadRequest(
            "vault categories cannot have a public link".into(),
        ));
    }

    for _ in 0..2 {
        let slug = new_public_slug();
        let updated = sqlx::query_as::<_, Category>(
            r#"
            UPDATE categories
            SET public_slug = $1, updated_at = now()
            WHERE id = $2
              AND owner_user_id = $3
              AND is_vault = FALSE
              AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(&slug)
        .bind(category_id)
        .bind(owner_user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                Error::Conflict("public slug collision".into())
            } else {
                Error::msg(e.to_string())
            }
        })?;

        if let Some(row) = updated {
            return Ok(to_public_link_view(row.public_slug.as_deref()));
        }
    }

    Err(Error::msg("failed to rotate public link"))
}

/// Owner: disable / revoke public link.
pub async fn disable_category_public_link(
    pool: &PgPool,
    owner_user_id: Uuid,
    category_id: Uuid,
) -> Result<CategoryPublicLinkView> {
    require_category_owner(pool, owner_user_id, category_id).await?;
    sqlx::query(
        r#"
        UPDATE categories
        SET public_slug = NULL, updated_at = now()
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(category_id)
    .bind(owner_user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(to_public_link_view(None))
}

/// Anonymous: resolve published list by slug (unlisted).
pub async fn get_public_list_by_slug(pool: &PgPool, slug: &str) -> Result<PublicListView> {
    let slug = slug.trim();
    if slug.is_empty() || !slug.starts_with(PUBLIC_SLUG_PREFIX) {
        return Err(Error::NotFound);
    }

    let cat = sqlx::query_as::<_, Category>(
        r#"
        SELECT * FROM categories
        WHERE public_slug = $1
          AND is_vault = FALSE
          AND deleted_at IS NULL
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    let owner = find_user_by_id(pool, cat.owner_user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let owner_display_name = user_share_label(&owner);

    let rows = sqlx::query_as::<_, linkdean_core::Bookmark>(
        r#"
        SELECT b.* FROM bookmarks b
        WHERE b.is_vault = FALSE
          AND b.deleted_at IS NULL
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
    .bind(cat.id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut bookmarks = Vec::with_capacity(rows.len());
    for b in rows {
        let tags = tags_for_bookmark(pool, b.id).await?;
        bookmarks.push(PublicBookmarkView {
            id: b.id,
            url: b.url,
            title: b.title,
            description: b.description,
            favicon_url: b.favicon_url,
            preview_image_url: b.preview_image_url,
            has_preview: b.has_preview,
            tags,
        });
    }

    Ok(PublicListView {
        name: cat.name,
        icon: cat.icon,
        owner_display_name,
        bookmarks,
    })
}
