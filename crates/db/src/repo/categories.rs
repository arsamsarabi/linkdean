use linkdean_core::{
    BookmarkView, Category, CategoryAccess, CategoryView, CreateBookmark, CreateCategory, Error,
    ForkCategory, Result, UpdateCategory,
};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::bookmarks::{bookmark_to_view, create_bookmark, get_bookmark_raw};
use super::shares::{
    bookmark_can_read, category_access, list_shared_categories, owned_shared_out_invitees,
    require_category_owner, require_category_read, require_category_write,
};

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
    out.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.name.cmp(&b.name)));
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

    let cat = sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (owner_user_id, parent_id, name, is_hidden, is_vault)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(owner_user_id)
    .bind(req.parent_id)
    .bind(req.name)
    .bind(req.is_hidden)
    .bind(req.is_vault)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let access = category_access(pool, user_id, cat.id).await?;
    Ok(CategoryView::from_category(cat, access))
}

pub async fn update_category(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateCategory,
) -> Result<CategoryView> {
    let (mut cat, access) = require_category_write(pool, user_id, id).await?;
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
    let updated = sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET name = $1, parent_id = $2, is_hidden = $3, position = $4, updated_at = now()
        WHERE id = $5
        RETURNING *
        "#,
    )
    .bind(&cat.name)
    .bind(cat.parent_id)
    .bind(cat.is_hidden)
    .bind(cat.position)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let access = category_access(pool, user_id, updated.id).await?;
    Ok(CategoryView::from_category(updated, access))
}

/// Soft-delete category subtree (descendants included). Owner-only.
pub async fn delete_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    require_category_owner(pool, user_id, id).await?;
    let cat = get_category_by_id(pool, id).await?;
    if cat.deleted_at.is_some() {
        return Ok(());
    }
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
    if !owned.iter().any(|c| c.id == id) {
        return Err(Error::NotFound);
    }
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
    Ok(())
}

pub async fn purge_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    require_category_owner(pool, user_id, id).await?;
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
