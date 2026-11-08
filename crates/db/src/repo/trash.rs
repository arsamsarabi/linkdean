use linkdean_core::{Error, Result, TrashItem, TrashKind};
use sqlx::AssertSqlSafe;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct TrashRow {
    id: Uuid,
    title: String,
    deleted_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_trash(pool: &PgPool, user_id: Uuid) -> Result<Vec<TrashItem>> {
    let bookmarks = sqlx::query_as::<_, TrashRow>(
        r#"
        SELECT id,
               CASE
                   WHEN is_vault THEN 'Vault item'
                   WHEN title <> '' THEN title
                   ELSE url
               END AS title,
               deleted_at
        FROM bookmarks
        WHERE owner_user_id = $1 AND deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let categories = sqlx::query_as::<_, TrashRow>(
        r#"
        SELECT id, CASE WHEN is_vault THEN 'Vault folder' ELSE name END AS title, deleted_at
        FROM categories
        WHERE owner_user_id = $1 AND deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let dashboards = sqlx::query_as::<_, TrashRow>(
        r#"
        SELECT id, title, deleted_at
        FROM dashboards
        WHERE owner_user_id = $1 AND deleted_at IS NOT NULL
        ORDER BY deleted_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(bookmarks.len() + categories.len() + dashboards.len());
    for r in bookmarks {
        out.push(TrashItem {
            kind: TrashKind::Bookmark,
            id: r.id,
            title: r.title,
            deleted_at: r.deleted_at,
        });
    }
    for r in categories {
        out.push(TrashItem {
            kind: TrashKind::Category,
            id: r.id,
            title: r.title,
            deleted_at: r.deleted_at,
        });
    }
    for r in dashboards {
        out.push(TrashItem {
            kind: TrashKind::Dashboard,
            id: r.id,
            title: r.title,
            deleted_at: r.deleted_at,
        });
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.deleted_at));
    Ok(out)
}

/// What one retention pass removed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PurgedTrash {
    /// Bookmarks removed from the database. Their search documents and preview files
    /// live outside it, so the caller removes those.
    pub bookmark_ids: Vec<Uuid>,
    pub categories: u64,
    pub dashboards: u64,
}

/// Hard-delete soft-deleted rows past each owner's retention window (30 days with no
/// settings row). Owners under an active ban keep their trash.
pub async fn purge_expired_trash(pool: &PgPool) -> Result<PurgedTrash> {
    // `{table}` and `{alias}` come from the fixed list below, never from input.
    fn sql(table: &str, alias: &str) -> String {
        format!(
            r#"
            DELETE FROM {table} {alias}
            USING users u
            LEFT JOIN user_settings s ON s.user_id = u.id
            WHERE {alias}.owner_user_id = u.id
              AND {alias}.deleted_at IS NOT NULL
              AND {alias}.deleted_at
                  < now() - make_interval(days => GREATEST(COALESCE(s.trash_retention_days, 30), 1))
              AND NOT EXISTS (
                  SELECT 1 FROM user_bans b
                  WHERE b.user_id = u.id
                    AND b.lifted_at IS NULL
                    AND (b.ends_at IS NULL OR b.ends_at > now())
              )
            RETURNING {alias}.id
            "#
        )
    }

    let bookmark_ids: Vec<Uuid> = sqlx::query_scalar(AssertSqlSafe(sql("bookmarks", "bm")))
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    // Live folders under expiring ones move to the top level first; the purge's cascade
    // would otherwise take them along.
    let expiring: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT c.id
        FROM categories c
        JOIN users u ON u.id = c.owner_user_id
        LEFT JOIN user_settings s ON s.user_id = u.id
        WHERE c.deleted_at IS NOT NULL
          AND c.deleted_at
              < now() - make_interval(days => GREATEST(COALESCE(s.trash_retention_days, 30), 1))
          AND NOT EXISTS (
              SELECT 1 FROM user_bans b
              WHERE b.user_id = u.id
                AND b.lifted_at IS NULL
                AND (b.ends_at IS NULL OR b.ends_at > now())
          )
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    super::categories::rescue_live_children(pool, &expiring).await?;
    let categories: Vec<Uuid> = sqlx::query_scalar(AssertSqlSafe(sql("categories", "c")))
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let dashboards: Vec<Uuid> = sqlx::query_scalar(AssertSqlSafe(sql("dashboards", "d")))
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    Ok(PurgedTrash {
        bookmark_ids,
        categories: categories.len() as u64,
        dashboards: dashboards.len() as u64,
    })
}

/// The vault a trashed item belongs to, if any. Purging vault content needs that vault
/// unlocked.
pub async fn trashed_item_vault(
    pool: &PgPool,
    user_id: Uuid,
    kind: TrashKind,
    id: Uuid,
) -> Result<Option<Uuid>> {
    let sql = match kind {
        TrashKind::Bookmark => {
            "SELECT vault_id FROM bookmarks WHERE id = $1 AND owner_user_id = $2"
        }
        TrashKind::Category => {
            "SELECT vault_id FROM categories WHERE id = $1 AND owner_user_id = $2"
        }
        TrashKind::Dashboard => return Ok(None),
    };
    let row: Option<Option<Uuid>> = sqlx::query_scalar(sql)
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.flatten())
}
