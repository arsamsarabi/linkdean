use linkdean_core::{
    Dashboard, DashboardItem, DashboardItemView, DashboardSection, DashboardSectionView,
    DashboardView, Error, PutDashboardLayout, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::bookmarks::{bookmark_to_view, get_bookmark_raw};
use super::shares::bookmark_can_read;

pub async fn ensure_default_dashboard(pool: &PgPool, user_id: Uuid) -> Result<Dashboard> {
    if let Some(d) = get_default_dashboard_row(pool, user_id).await? {
        return Ok(d);
    }
    sqlx::query_as::<_, Dashboard>(
        r#"
        INSERT INTO dashboards (owner_user_id, title, is_default)
        VALUES ($1, 'Home', TRUE)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

async fn get_default_dashboard_row(pool: &PgPool, user_id: Uuid) -> Result<Option<Dashboard>> {
    sqlx::query_as::<_, Dashboard>(
        r#"
        SELECT * FROM dashboards
        WHERE owner_user_id = $1 AND is_default = TRUE
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_default_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let dash = ensure_default_dashboard(pool, user_id).await?;
    load_dashboard_view(pool, user_id, dash, vault_key).await
}

async fn load_dashboard_view(
    pool: &PgPool,
    user_id: Uuid,
    dash: Dashboard,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let sections = sqlx::query_as::<_, DashboardSection>(
        r#"
        SELECT * FROM dashboard_sections
        WHERE dashboard_id = $1
        ORDER BY position, created_at
        "#,
    )
    .bind(dash.id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut section_views = Vec::with_capacity(sections.len());
    for sec in sections {
        let items = sqlx::query_as::<_, DashboardItem>(
            r#"
            SELECT * FROM dashboard_items
            WHERE section_id = $1
            ORDER BY position, created_at
            "#,
        )
        .bind(sec.id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let mut item_views = Vec::with_capacity(items.len());
        for item in items {
            if !bookmark_can_read(pool, user_id, item.bookmark_id).await? {
                continue;
            }
            let raw = match get_bookmark_raw(pool, user_id, item.bookmark_id).await {
                Ok(b) => b,
                Err(Error::NotFound) => continue,
                Err(e) => return Err(e),
            };
            let key = if raw.is_vault && raw.owner_user_id == user_id {
                vault_key
            } else {
                None
            };
            let Some(view) = bookmark_to_view(pool, raw, key, user_id).await? else {
                continue;
            };
            item_views.push(DashboardItemView {
                id: item.id,
                position: item.position,
                bookmark: view,
            });
        }

        section_views.push(DashboardSectionView {
            id: sec.id,
            title: sec.title,
            position: sec.position,
            items: item_views,
        });
    }

    Ok(DashboardView {
        id: dash.id,
        title: dash.title,
        is_default: dash.is_default,
        sections: section_views,
        created_at: dash.created_at,
        updated_at: dash.updated_at,
    })
}

pub async fn put_default_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    req: PutDashboardLayout,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let dash = ensure_default_dashboard(pool, user_id).await?;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Home");

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        UPDATE dashboards
        SET title = $1, updated_at = now()
        WHERE id = $2 AND owner_user_id = $3
        "#,
    )
    .bind(title)
    .bind(dash.id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query("DELETE FROM dashboard_sections WHERE dashboard_id = $1")
        .bind(dash.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    for (si, section) in req.sections.iter().enumerate() {
        let section_title = section.title.trim();
        if section_title.is_empty() {
            return Err(Error::BadRequest("section title required".into()));
        }
        let sec = sqlx::query_as::<_, DashboardSection>(
            r#"
            INSERT INTO dashboard_sections (dashboard_id, title, position)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(dash.id)
        .bind(section_title)
        .bind(si as i32)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let mut seen = std::collections::HashSet::new();
        for (ii, bookmark_id) in section.bookmark_ids.iter().enumerate() {
            if !seen.insert(*bookmark_id) {
                continue;
            }
            if !bookmark_can_read(pool, user_id, *bookmark_id).await? {
                return Err(Error::BadRequest(
                    "bookmark not found or not readable".into(),
                ));
            }
            sqlx::query(
                r#"
                INSERT INTO dashboard_items (section_id, bookmark_id, position)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(sec.id)
            .bind(bookmark_id)
            .bind(ii as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    get_default_dashboard(pool, user_id, vault_key).await
}
