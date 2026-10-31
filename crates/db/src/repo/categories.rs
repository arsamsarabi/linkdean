use linkdean_core::{
    Category, CategoryAccess, CategoryView, CreateCategory, Error, Result, UpdateCategory,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use super::shares::{
    category_access, list_shared_categories, require_category_owner, require_category_write,
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
) -> Result<Vec<CategoryView>> {
    let owned = sqlx::query_as::<_, Category>(
        r#"
        SELECT * FROM categories
        WHERE owner_user_id = $1
          AND ($2 OR is_hidden = FALSE)
          AND ($3 OR is_vault = FALSE)
        ORDER BY position, name
        "#,
    )
    .bind(user_id)
    .bind(include_hidden)
    .bind(include_vault)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut by_id: HashMap<Uuid, CategoryView> = owned
        .into_iter()
        .map(|c| {
            let id = c.id;
            (id, CategoryView::from_category(c, CategoryAccess::Owner))
        })
        .collect();

    let shared = list_shared_categories(pool, user_id, include_hidden).await?;
    for (cat, access) in shared {
        by_id
            .entry(cat.id)
            .or_insert_with(|| CategoryView::from_category(cat, access));
    }

    let mut out: Vec<CategoryView> = by_id.into_values().collect();
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
    Ok(CategoryView::from_category(cat, access))
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

pub async fn delete_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    // Owner-only: invitees cannot delete shared folders.
    require_category_owner(pool, user_id, id).await?;
    let res = sqlx::query("DELETE FROM categories WHERE id = $1 AND owner_user_id = $2")
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
