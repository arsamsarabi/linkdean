//! Sections that fill themselves (Latest, Most opened, Recently visited) and the per-person
//! bookmark opens they read. They draw only on the dashboard owner's own non-vault
//! bookmarks, which is why a dashboard that has one can never be shared.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use linkdean_core::{
    Bookmark, DashboardOpenWindow, DashboardSectionKind, DashboardSectionSettings,
    DashboardSectionView, DashboardStats, Error, Result,
};
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;

/// The host of `b.url` in lower case without `www.`, for the website source.
const HOST_SQL: &str = r"regexp_replace(
    lower(substring(b.url from '^[A-Za-z][A-Za-z0-9+.-]*://(?:[^@/]*@)?([^/:?#]+)')),
    '^www\.', '')";

/// The owner's bookmarks for a section that fills itself, in the section's order, at most
/// `item_limit`. Pinned sections have none.
pub async fn fill_dashboard_section(
    pool: &PgPool,
    owner_user_id: Uuid,
    settings: &DashboardSectionSettings,
) -> Result<Vec<Bookmark>> {
    let (join, order) = match (settings.kind, settings.open_window) {
        (DashboardSectionKind::Pinned, _) => return Ok(Vec::new()),
        (DashboardSectionKind::Latest, _) => ("", "b.created_at DESC, b.id"),
        (DashboardSectionKind::MostOpened, DashboardOpenWindow::AllTime) => (
            "JOIN bookmark_open_stats s ON s.bookmark_id = b.id AND s.user_id = $1",
            "s.open_count DESC, s.last_opened_at DESC, b.id",
        ),
        (DashboardSectionKind::MostOpened, DashboardOpenWindow::Last30Days) => (
            r#"JOIN (
                SELECT bookmark_id, COUNT(*) AS opens, MAX(opened_at) AS last_open
                FROM bookmark_opens
                WHERE user_id = $1 AND opened_at > now() - interval '30 days'
                GROUP BY bookmark_id
            ) o ON o.bookmark_id = b.id"#,
            "o.opens DESC, o.last_open DESC, b.id",
        ),
        (DashboardSectionKind::RecentlyVisited, _) => (
            "JOIN bookmark_open_stats s ON s.bookmark_id = b.id AND s.user_id = $1",
            "s.last_opened_at DESC, b.id",
        ),
    };
    let source = &settings.source;
    let sql = format!(
        r#"
        WITH RECURSIVE folder AS (
            SELECT id FROM categories WHERE id = $2 AND owner_user_id = $1
            UNION ALL
            SELECT c.id FROM categories c JOIN folder f ON c.parent_id = f.id
        )
        SELECT b.* FROM bookmarks b
        {join}
        LEFT JOIN bookmark_flags fl ON fl.bookmark_id = b.id AND fl.user_id = $1
        WHERE b.owner_user_id = $1
          AND b.deleted_at IS NULL
          AND b.is_vault = FALSE
          AND (
            $2::uuid IS NULL
            OR b.category_id IN (SELECT id FROM folder)
            OR EXISTS (
              SELECT 1 FROM bookmark_categories bc
              WHERE bc.bookmark_id = b.id AND bc.category_id IN (SELECT id FROM folder)
            )
          )
          AND (
            $3::text IS NULL
            OR EXISTS (
              SELECT 1 FROM bookmark_tags bt JOIN tags t ON t.id = bt.tag_id
              WHERE bt.bookmark_id = b.id AND lower(t.name) = lower($3)
            )
          )
          AND ($4::text IS NULL OR {HOST_SQL} = $4 OR {HOST_SQL} LIKE '%.' || $4)
          AND (NOT $5 OR COALESCE(fl.is_favorite, FALSE))
          AND (NOT $6 OR COALESCE(fl.is_unread, FALSE))
        ORDER BY {order}
        LIMIT $7
        "#
    );
    sqlx::query_as::<_, Bookmark>(AssertSqlSafe(sql))
        .bind(owner_user_id)
        .bind(source.category_id)
        .bind(source.tag.as_deref())
        .bind(source.site.as_deref())
        .bind(source.favorites)
        .bind(source.unread)
        .bind(i64::from(settings.item_limit))
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

/// Whether the dashboard is shared with anyone, or has an invite waiting to be claimed.
pub async fn dashboard_has_any_share(pool: &PgPool, dashboard_id: Uuid) -> Result<bool> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (SELECT 1 FROM dashboard_shares WHERE dashboard_id = $1)
            OR EXISTS (
              SELECT 1 FROM pending_share_invites
              WHERE subject_type = 'dashboard' AND subject_id = $1
                AND claimed_at IS NULL AND revoked_at IS NULL
            )
        "#,
    )
    .bind(dashboard_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

/// Whether the dashboard has a section that fills itself.
pub async fn dashboard_has_self_filling_section(pool: &PgPool, dashboard_id: Uuid) -> Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM dashboard_sections WHERE dashboard_id = $1 AND kind <> 'pinned')",
    )
    .bind(dashboard_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub const SELF_FILLING_SHARED_MESSAGE: &str =
    "sections that fill themselves are only for dashboards you have not shared";

/// Records that `user_id` opened a bookmark they can read. Vault bookmarks are never
/// recorded. Opens older than 30 days are pruned as new ones come in.
pub async fn record_bookmark_open(pool: &PgPool, user_id: Uuid, bookmark_id: Uuid) -> Result<()> {
    let raw = super::bookmarks::get_bookmark_raw(pool, user_id, bookmark_id).await?;
    if raw.is_vault || raw.deleted_at.is_some() {
        return Ok(());
    }
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO bookmark_open_stats (user_id, bookmark_id, open_count, last_opened_at)
        VALUES ($1, $2, 1, now())
        ON CONFLICT (user_id, bookmark_id)
        DO UPDATE SET open_count = bookmark_open_stats.open_count + 1, last_opened_at = now()
        "#,
    )
    .bind(user_id)
    .bind(bookmark_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query("INSERT INTO bookmark_opens (user_id, bookmark_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(bookmark_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        "DELETE FROM bookmark_opens WHERE user_id = $1 AND opened_at < now() - interval '30 days'",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))
}

/// Fills each item's open count and last open from the viewer's own opens. A section that
/// counts the last 30 days shows that count; every other section shows the all-time count.
pub async fn attach_open_stats(
    pool: &PgPool,
    user_id: Uuid,
    sections: &mut [DashboardSectionView],
) -> Result<()> {
    let ids: Vec<Uuid> = sections
        .iter()
        .flat_map(|s| s.items.iter().map(|i| i.bookmark.id))
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let rows: Vec<(Uuid, i64, DateTime<Utc>, i64)> = sqlx::query_as(
        r#"
        SELECT s.bookmark_id, s.open_count::bigint, s.last_opened_at,
               (SELECT COUNT(*) FROM bookmark_opens o
                WHERE o.user_id = s.user_id AND o.bookmark_id = s.bookmark_id
                  AND o.opened_at > now() - interval '30 days')
        FROM bookmark_open_stats s
        WHERE s.user_id = $1 AND s.bookmark_id = ANY($2)
        "#,
    )
    .bind(user_id)
    .bind(&ids)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let by_id: HashMap<Uuid, (i64, DateTime<Utc>, i64)> = rows
        .into_iter()
        .map(|(id, all, last, recent)| (id, (all, last, recent)))
        .collect();
    for section in sections.iter_mut() {
        let recent_window = section.settings.kind == DashboardSectionKind::MostOpened
            && section.settings.open_window == DashboardOpenWindow::Last30Days;
        for item in section.items.iter_mut() {
            if let Some((all, last, recent)) = by_id.get(&item.bookmark.id) {
                let count = if recent_window { *recent } else { *all };
                item.open_count = (count > 0).then_some(count);
                item.last_opened_at = Some(*last);
            }
        }
    }
    Ok(())
}

/// Counts over the viewer's own live bookmarks for the dashboard header.
pub async fn dashboard_stats(pool: &PgPool, user_id: Uuid) -> Result<DashboardStats> {
    let (total, saved_this_week, unread, favorites, opened_today): (i64, i64, i64, i64, i64) =
        sqlx::query_as(
            r#"
            SELECT
              (SELECT COUNT(*) FROM bookmarks b
               WHERE b.owner_user_id = $1 AND b.deleted_at IS NULL),
              (SELECT COUNT(*) FROM bookmarks b
               WHERE b.owner_user_id = $1 AND b.deleted_at IS NULL
                 AND b.created_at > now() - interval '7 days'),
              (SELECT COUNT(*) FROM bookmark_flags f JOIN bookmarks b ON b.id = f.bookmark_id
               WHERE f.user_id = $1 AND f.is_unread AND b.deleted_at IS NULL
                 AND b.owner_user_id = $1),
              (SELECT COUNT(*) FROM bookmark_flags f JOIN bookmarks b ON b.id = f.bookmark_id
               WHERE f.user_id = $1 AND f.is_favorite AND b.deleted_at IS NULL
                 AND b.owner_user_id = $1),
              (SELECT COUNT(*) FROM bookmark_opens o
               WHERE o.user_id = $1 AND o.opened_at > now() - interval '24 hours')
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(DashboardStats {
        total,
        saved_this_week,
        unread,
        favorites,
        opened_today,
    })
}
