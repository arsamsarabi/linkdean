use linkdean_core::{Category, CreateCategory, Error, Result, UpdateCategory};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_categories(
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

pub async fn get_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Category> {
    sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = $1 AND owner_user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

pub async fn create_category(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateCategory,
) -> Result<Category> {
    if let Some(pid) = req.parent_id {
        let parent = get_category(pool, user_id, pid).await?;
        if parent.is_vault && !req.is_vault {
            return Err(Error::BadRequest(
                "non-vault category cannot nest under vault".into(),
            ));
        }
    }
    sqlx::query_as::<_, Category>(
        r#"
        INSERT INTO categories (owner_user_id, parent_id, name, is_hidden, is_vault)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.parent_id)
    .bind(req.name)
    .bind(req.is_hidden)
    .bind(req.is_vault)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn update_category(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: UpdateCategory,
) -> Result<Category> {
    let mut cat = get_category(pool, user_id, id).await?;
    if let Some(name) = req.name {
        cat.name = name;
    }
    if let Some(parent) = req.parent_id {
        cat.parent_id = parent;
    }
    if let Some(h) = req.is_hidden {
        cat.is_hidden = h;
    }
    if let Some(p) = req.position {
        cat.position = p;
    }
    sqlx::query_as::<_, Category>(
        r#"
        UPDATE categories
        SET name = $1, parent_id = $2, is_hidden = $3, position = $4, updated_at = now()
        WHERE id = $5 AND owner_user_id = $6
        RETURNING *
        "#,
    )
    .bind(&cat.name)
    .bind(cat.parent_id)
    .bind(cat.is_hidden)
    .bind(cat.position)
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn delete_category(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
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
