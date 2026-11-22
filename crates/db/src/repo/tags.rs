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

/// List tags in a team space (caller must already authorize Viewer+).
pub async fn list_team_tags(pool: &PgPool, team_id: Uuid) -> Result<Vec<Tag>> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE team_id = $1 ORDER BY name")
        .bind(team_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn ensure_tags(pool: &PgPool, user_id: Uuid, names: &[String]) -> Result<Vec<Tag>> {
    ensure_tags_in_space(pool, Some(user_id), None, names).await
}

pub async fn ensure_tags_in_space(
    pool: &PgPool,
    owner_user_id: Option<Uuid>,
    team_id: Option<Uuid>,
    names: &[String],
) -> Result<Vec<Tag>> {
    if owner_user_id.is_some() == team_id.is_some() {
        return Err(Error::BadRequest(
            "tags need either a personal owner or a team".into(),
        ));
    }
    let mut out = Vec::new();
    // A comma separates tags everywhere (tag inputs, imports, the `tag=` filter), so it is never
    // part of one.
    // shortcut: a tag saved with a comma before 0.0.13 keeps it until edited off its bookmarks;
    // split them in a migration if any turn up.
    for name in names.iter().flat_map(|n| n.split(',')) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let tag = if let Some(uid) = owner_user_id {
            sqlx::query_as::<_, Tag>(
                r#"
                INSERT INTO tags (owner_user_id, team_id, name)
                VALUES ($1, NULL, $2)
                ON CONFLICT (owner_user_id, name) WHERE owner_user_id IS NOT NULL
                DO UPDATE SET name = EXCLUDED.name
                RETURNING *
                "#,
            )
            .bind(uid)
            .bind(name)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?
        } else {
            let tid = team_id.expect("team_id xor checked");
            sqlx::query_as::<_, Tag>(
                r#"
                INSERT INTO tags (owner_user_id, team_id, name)
                VALUES (NULL, $1, $2)
                ON CONFLICT (team_id, name) WHERE team_id IS NOT NULL
                DO UPDATE SET name = EXCLUDED.name
                RETURNING *
                "#,
            )
            .bind(tid)
            .bind(name)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?
        };
        out.push(tag);
    }
    Ok(out)
}

/// Delete tags no bookmark uses, so renamed and removed tags do not pile up. Tags made in
/// the last hour are left alone: a bookmark save creates its tags before linking them.
pub async fn reap_unused_tags(pool: &PgPool) -> Result<u64> {
    let res = sqlx::query(
        r#"
        DELETE FROM tags t
        WHERE t.created_at < now() - interval '1 hour'
          AND NOT EXISTS (SELECT 1 FROM bookmark_tags bt WHERE bt.tag_id = t.id)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(res.rows_affected())
}
