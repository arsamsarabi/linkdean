use linkdean_core::{
    Bookmark, CategoryAccess, CreateDashboard, Dashboard, DashboardItem, DashboardItemView,
    DashboardSection, DashboardSectionView, DashboardSummary, DashboardView, Error, PatchDashboard,
    PutDashboardLayout, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::bookmarks::bookmark_to_view;
use super::dashboard_shares::{
    dashboard_shared_out, list_shared_dashboards, require_dashboard_owner, require_dashboard_read,
    require_dashboard_write,
};
use super::shares::bookmark_can_read;

async fn fetch_bookmark_row(pool: &PgPool, id: Uuid) -> Result<Option<Bookmark>> {
    sqlx::query_as::<_, Bookmark>("SELECT * FROM bookmarks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn ensure_default_dashboard(pool: &PgPool, user_id: Uuid) -> Result<Dashboard> {
    if let Some(d) = get_default_dashboard_row(pool, user_id).await? {
        return Ok(d);
    }
    // Prefer any existing row (e.g. after delete cleared the pin) before inserting.
    if let Some(any) = list_dashboard_rows(pool, user_id).await?.into_iter().next() {
        return pin_dashboard_row(pool, user_id, any.id).await;
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
        WHERE owner_user_id = $1 AND is_default = TRUE AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

async fn list_dashboard_rows(pool: &PgPool, user_id: Uuid) -> Result<Vec<Dashboard>> {
    list_dashboard_rows_filtered(pool, user_id, false).await
}

async fn list_dashboard_rows_filtered(
    pool: &PgPool,
    user_id: Uuid,
    include_deleted: bool,
) -> Result<Vec<Dashboard>> {
    sqlx::query_as::<_, Dashboard>(
        r#"
        SELECT * FROM dashboards
        WHERE owner_user_id = $1
          AND ($2 OR deleted_at IS NULL)
        ORDER BY is_default DESC, title ASC, created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(include_deleted)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

fn owned_summary(d: Dashboard, shared_out: bool) -> DashboardSummary {
    DashboardSummary {
        id: d.id,
        owner_user_id: d.owner_user_id,
        title: d.title,
        is_default: d.is_default,
        access: CategoryAccess::Owner,
        shared_out,
        owner_display_name: None,
        deleted_at: d.deleted_at,
        created_at: d.created_at,
        updated_at: d.updated_at,
    }
}

fn shared_summary(d: Dashboard, access: CategoryAccess, owner_display_name: Option<String>) -> DashboardSummary {
    DashboardSummary {
        id: d.id,
        owner_user_id: d.owner_user_id,
        title: d.title,
        // Invitee cannot pin someone else's board as their default.
        is_default: false,
        access,
        shared_out: false,
        owner_display_name,
        deleted_at: d.deleted_at,
        created_at: d.created_at,
        updated_at: d.updated_at,
    }
}

pub async fn list_dashboards(pool: &PgPool, user_id: Uuid) -> Result<Vec<DashboardSummary>> {
    list_dashboards_filtered(pool, user_id, false).await
}

pub async fn list_dashboards_filtered(
    pool: &PgPool,
    user_id: Uuid,
    include_deleted: bool,
) -> Result<Vec<DashboardSummary>> {
    let _ = ensure_default_dashboard(pool, user_id).await?;
    let owned = list_dashboard_rows_filtered(pool, user_id, include_deleted).await?;
    let mut out = Vec::with_capacity(owned.len());
    for d in owned {
        let shared_out = dashboard_shared_out(pool, d.id).await?;
        out.push(owned_summary(d, shared_out));
    }
    for (d, access) in list_shared_dashboards(pool, user_id).await? {
        if d.deleted_at.is_some() {
            continue;
        }
        let owner_display_name =
            super::users::owner_display_name_for_viewer(pool, d.owner_user_id, user_id).await?;
        out.push(shared_summary(d, access, owner_display_name));
    }
    Ok(out)
}

pub async fn get_default_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let dash = ensure_default_dashboard(pool, user_id).await?;
    load_dashboard_view(pool, user_id, dash, CategoryAccess::Owner, vault_key).await
}

pub async fn get_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let (dash, access) = require_dashboard_read(pool, user_id, id).await?;
    load_dashboard_view(pool, user_id, dash, access, vault_key).await
}

async fn load_dashboard_view(
    pool: &PgPool,
    user_id: Uuid,
    dash: Dashboard,
    access: CategoryAccess,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let shared_out = if access.is_owner() {
        dashboard_shared_out(pool, dash.id).await?
    } else {
        false
    };
    let is_default = if access.is_owner() {
        dash.is_default
    } else {
        false
    };
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
            let Some(raw) = fetch_bookmark_row(pool, item.bookmark_id).await? else {
                continue;
            };
            // Vault pins: bookmark owner only. Non-vault: any dashboard viewer may glance the pin.
            if raw.is_vault && raw.owner_user_id != user_id {
                continue;
            }
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

    let owner_display_name = if access.is_owner() {
        None
    } else {
        super::users::owner_display_name_for_viewer(pool, dash.owner_user_id, user_id).await?
    };

    Ok(DashboardView {
        id: dash.id,
        owner_user_id: dash.owner_user_id,
        title: dash.title,
        is_default,
        sections: section_views,
        access,
        shared_out,
        owner_display_name,
        deleted_at: dash.deleted_at,
        created_at: dash.created_at,
        updated_at: dash.updated_at,
    })
}

pub async fn create_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateDashboard,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let _ = ensure_default_dashboard(pool, user_id).await?;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("New dashboard");

    let dash = sqlx::query_as::<_, Dashboard>(
        r#"
        INSERT INTO dashboards (owner_user_id, title, is_default)
        VALUES ($1, $2, FALSE)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    load_dashboard_view(pool, user_id, dash, CategoryAccess::Owner, vault_key).await
}

pub async fn put_default_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    req: PutDashboardLayout,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let dash = ensure_default_dashboard(pool, user_id).await?;
    put_dashboard(pool, user_id, dash.id, req, vault_key).await
}

pub async fn put_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: PutDashboardLayout,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    let (dash, _) = require_dashboard_write(pool, user_id, id).await?;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(dash.title.as_str());

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        UPDATE dashboards
        SET title = $1, updated_at = now()
        WHERE id = $2
        "#,
    )
    .bind(title)
    .bind(dash.id)
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
            // Owner / readers of the bookmark, or write invitees rearranging owner's non-vault pins.
            let readable = bookmark_can_read(pool, user_id, *bookmark_id).await?;
            let owner_pin = match fetch_bookmark_row(pool, *bookmark_id).await? {
                Some(b) if !b.is_vault && b.owner_user_id == dash.owner_user_id => true,
                _ => false,
            };
            if !readable && !owner_pin {
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

    get_dashboard(pool, user_id, id, vault_key).await
}

async fn pin_dashboard_row(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Dashboard> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        UPDATE dashboards
        SET is_default = FALSE, updated_at = now()
        WHERE owner_user_id = $1 AND is_default = TRUE
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let dash = sqlx::query_as::<_, Dashboard>(
        r#"
        UPDATE dashboards
        SET is_default = TRUE, updated_at = now()
        WHERE id = $1 AND owner_user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    tx.commit()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    Ok(dash)
}

pub async fn patch_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: PatchDashboard,
    vault_key: Option<&[u8; 32]>,
) -> Result<DashboardView> {
    // Pin is owner-only. Title may be updated by write invitees (same as put layout).
    if req.is_default.is_some() {
        let dash = require_dashboard_owner(pool, user_id, id).await?;
        if let Some(true) = req.is_default {
            let _ = pin_dashboard_row(pool, user_id, id).await?;
        } else if let Some(false) = req.is_default {
            if dash.is_default {
                return Err(Error::BadRequest(
                    "pin another dashboard instead of unpinning the only default".into(),
                ));
            }
        }
    } else {
        require_dashboard_write(pool, user_id, id).await?;
    }

    if let Some(ref title_raw) = req.title {
        let title = title_raw.trim();
        if title.is_empty() {
            return Err(Error::BadRequest("title required".into()));
        }
        sqlx::query(
            r#"
            UPDATE dashboards
            SET title = $1, updated_at = now()
            WHERE id = $2
            "#,
        )
        .bind(title)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }

    get_dashboard(pool, user_id, id, vault_key).await
}

pub async fn delete_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> Result<Vec<DashboardSummary>> {
    let dash = require_dashboard_owner(pool, user_id, id).await?;
    if dash.deleted_at.is_some() {
        return list_dashboards(pool, user_id).await;
    }
    let was_default = dash.is_default;

    sqlx::query(
        r#"
        UPDATE dashboards
        SET deleted_at = now(), is_default = FALSE, updated_at = now()
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let remaining = list_dashboard_rows(pool, user_id).await?;
    if remaining.is_empty() {
        let _ = ensure_default_dashboard(pool, user_id).await?;
    } else if was_default {
        let next = remaining[0].id;
        let _ = pin_dashboard_row(pool, user_id, next).await?;
    }

    list_dashboards(pool, user_id).await
}

pub async fn restore_dashboard(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let res = sqlx::query(
        r#"
        UPDATE dashboards
        SET deleted_at = NULL, updated_at = now()
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

pub async fn purge_dashboard(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let res = sqlx::query(
        r#"
        DELETE FROM dashboards
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
