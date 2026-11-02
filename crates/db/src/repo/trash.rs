use chrono::{Duration, Utc};
use linkdean_core::{Error, Result, TrashItem, TrashKind};
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
        SELECT id, name AS title, deleted_at
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

/// Hard-delete soft-deleted rows past each owner's retention window.
pub async fn purge_expired_trash(pool: &PgPool) -> Result<(u64, u64, u64)> {
    let settings: Vec<(Uuid, i32)> = sqlx::query_as(
        r#"
        SELECT user_id, trash_retention_days FROM user_settings s
        WHERE NOT EXISTS (
            SELECT 1 FROM user_bans b
            WHERE b.user_id = s.user_id
              AND b.lifted_at IS NULL
              AND (b.ends_at IS NULL OR b.ends_at > now())
        )
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut bookmarks = 0u64;
    let mut categories = 0u64;
    let mut dashboards = 0u64;

    for (user_id, days) in settings {
        // A cutoff before the calendar's range means nothing is old enough to purge.
        let Some(cutoff) = Utc::now().checked_sub_signed(Duration::days(i64::from(days.max(1))))
        else {
            continue;
        };
        let b = sqlx::query(
            r#"
            DELETE FROM bookmarks
            WHERE owner_user_id = $1 AND deleted_at IS NOT NULL AND deleted_at < $2
            "#,
        )
        .bind(user_id)
        .bind(cutoff)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        bookmarks += b.rows_affected();

        let c = sqlx::query(
            r#"
            DELETE FROM categories
            WHERE owner_user_id = $1 AND deleted_at IS NOT NULL AND deleted_at < $2
            "#,
        )
        .bind(user_id)
        .bind(cutoff)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        categories += c.rows_affected();

        let d = sqlx::query(
            r#"
            DELETE FROM dashboards
            WHERE owner_user_id = $1 AND deleted_at IS NOT NULL AND deleted_at < $2
            "#,
        )
        .bind(user_id)
        .bind(cutoff)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        dashboards += d.rows_affected();
    }

    // Owners with no settings row yet: use default 30 days.
    let cutoff_default = Utc::now() - Duration::days(30);
    let b = sqlx::query(
        r#"
        DELETE FROM bookmarks b
        WHERE b.deleted_at IS NOT NULL
          AND b.deleted_at < $1
          AND NOT EXISTS (SELECT 1 FROM user_settings s WHERE s.user_id = b.owner_user_id)
        "#,
    )
    .bind(cutoff_default)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    bookmarks += b.rows_affected();

    let c = sqlx::query(
        r#"
        DELETE FROM categories c
        WHERE c.deleted_at IS NOT NULL
          AND c.deleted_at < $1
          AND NOT EXISTS (SELECT 1 FROM user_settings s WHERE s.user_id = c.owner_user_id)
        "#,
    )
    .bind(cutoff_default)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    categories += c.rows_affected();

    let d = sqlx::query(
        r#"
        DELETE FROM dashboards d
        WHERE d.deleted_at IS NOT NULL
          AND d.deleted_at < $1
          AND NOT EXISTS (SELECT 1 FROM user_settings s WHERE s.user_id = d.owner_user_id)
        "#,
    )
    .bind(cutoff_default)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    dashboards += d.rows_affected();

    Ok((bookmarks, categories, dashboards))
}
