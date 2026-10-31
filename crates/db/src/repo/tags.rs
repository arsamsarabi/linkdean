use linkdean_core::{Error, Result, Tag};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_tags(pool: &PgPool, user_id: Uuid) -> Result<Vec<Tag>> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE owner_user_id = $1 ORDER BY name")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn ensure_tags(pool: &PgPool, user_id: Uuid, names: &[String]) -> Result<Vec<Tag>> {
    let mut out = Vec::new();
    for name in names {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (owner_user_id, name)
            VALUES ($1, $2)
            ON CONFLICT (owner_user_id, name) DO UPDATE SET name = EXCLUDED.name
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        out.push(tag);
    }
    Ok(out)
}
