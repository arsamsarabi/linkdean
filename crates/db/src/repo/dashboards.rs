use linkdean_core::VaultKeyring;
use linkdean_core::{
    Bookmark, CategoryAccess, CreateDashboard, Dashboard, DashboardItem, DashboardItemView,
    DashboardSection, DashboardSectionView, DashboardSummary, DashboardView, Error, PatchDashboard,
    PutDashboardLayout, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::bookmarks::bookmark_to_view;
use super::dashboard_shares::{
    clear_shared_dashboard_pin, dashboard_shared_with, list_shared_dashboards,
    pin_shared_dashboard, pinned_shared_dashboard_id, require_dashboard_owner,
    require_dashboard_read, require_dashboard_write,
};
use super::shares::bookmark_can_read;

async fn fetch_bookmark_row(pool: &PgPool, id: Uuid) -> Result<Option<Bookmark>> {
    sqlx::query_as::<_, Bookmark>("SELECT * FROM bookmarks WHERE id = $1 AND deleted_at IS NULL")
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
    // Parallel first requests race here. The loser takes the row the winner made.
    let created = sqlx::query_as::<_, Dashboard>(
        r#"
        INSERT INTO dashboards (owner_user_id, title, is_default)
        VALUES ($1, 'Home', TRUE)
        ON CONFLICT (owner_user_id) WHERE is_default DO NOTHING
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    match created {
        Some(d) => Ok(d),
        None => get_default_dashboard_row(pool, user_id)
            .await?
            .ok_or_else(|| Error::msg("default dashboard missing after conflict")),
    }
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

fn owned_summary(d: Dashboard, shared_with: Vec<String>, is_default: bool) -> DashboardSummary {
    let shared_out = !shared_with.is_empty();
    DashboardSummary {
        id: d.id,
        owner_user_id: d.owner_user_id,
        title: d.title,
        is_default,
        access: CategoryAccess::Owner,
        shared_out,
        shared_with,
        owner_display_name: None,
        deleted_at: d.deleted_at,
        created_at: d.created_at,
        updated_at: d.updated_at,
    }
}

fn shared_summary(
    d: Dashboard,
    access: CategoryAccess,
    owner_display_name: Option<String>,
    is_default: bool,
) -> DashboardSummary {
    DashboardSummary {
        id: d.id,
        owner_user_id: d.owner_user_id,
        title: d.title,
        is_default,
        access,
        shared_out: false,
        shared_with: Vec::new(),
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
    let pinned = pinned_shared_dashboard_id(pool, user_id).await?;
    let mut out = Vec::with_capacity(owned.len());
    for d in owned {
        let shared_with = dashboard_shared_with(pool, d.id).await?;
        let is_default = d.is_default && pinned.is_none();
        out.push(owned_summary(d, shared_with, is_default));
    }
    for (d, access) in list_shared_dashboards(pool, user_id).await? {
        if d.deleted_at.is_some() {
            continue;
        }
        let owner_display_name =
            super::users::owner_display_name_for_viewer(pool, d.owner_user_id, user_id).await?;
        let is_default = pinned == Some(d.id);
        out.push(shared_summary(d, access, owner_display_name, is_default));
    }
    Ok(out)
}

pub async fn get_default_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    keyring: &VaultKeyring,
) -> Result<DashboardView> {
    if let Some(id) = pinned_shared_dashboard_id(pool, user_id).await? {
        return get_dashboard(pool, user_id, id, keyring).await;
    }
    let dash = ensure_default_dashboard(pool, user_id).await?;
    load_dashboard_view(pool, user_id, dash, CategoryAccess::Owner, keyring).await
}

pub async fn get_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    keyring: &VaultKeyring,
) -> Result<DashboardView> {
    let (dash, access) = require_dashboard_read(pool, user_id, id).await?;
    load_dashboard_view(pool, user_id, dash, access, keyring).await
}

/// Whether `user_id` may see this pin on a dashboard owned by `dashboard_owner`.
///
/// Sharing a dashboard shares the owner's own non-vault bookmarks pinned on it. Any other
/// pin shows only to people who can read the bookmark themselves, so a pin never outlives
/// a revoked folder share or re-shares someone else's bookmark. Vault pins show only to
/// their owner while the vault is unlocked.
async fn pin_visible_to(
    pool: &PgPool,
    user_id: Uuid,
    b: &Bookmark,
    dashboard_owner: Uuid,
    keyring: &VaultKeyring,
) -> Result<bool> {
    if b.is_vault {
        return Ok(b.owner_user_id == user_id && b.vault_id.and_then(|v| keyring.get(v)).is_some());
    }
    if b.owner_user_id == dashboard_owner {
        return Ok(true);
    }
    bookmark_can_read(pool, user_id, b.id).await
}

/// Whether `user_id` cannot see this pin: the bookmark is trashed, or [`pin_visible_to`] says no.
async fn pin_hidden_from(
    pool: &PgPool,
    user_id: Uuid,
    bookmark_id: Uuid,
    dashboard_owner: Uuid,
    keyring: &VaultKeyring,
) -> Result<bool> {
    Ok(match fetch_bookmark_row(pool, bookmark_id).await? {
        None => true,
        Some(b) => !pin_visible_to(pool, user_id, &b, dashboard_owner, keyring).await?,
    })
}

async fn load_dashboard_view(
    pool: &PgPool,
    user_id: Uuid,
    dash: Dashboard,
    access: CategoryAccess,
    keyring: &VaultKeyring,
) -> Result<DashboardView> {
    let shared_with = if access.is_owner() {
        dashboard_shared_with(pool, dash.id).await?
    } else {
        Vec::new()
    };
    let shared_out = !shared_with.is_empty();
    let pinned = pinned_shared_dashboard_id(pool, user_id).await?;
    let is_default = if access.is_owner() {
        dash.is_default && pinned.is_none()
    } else {
        pinned == Some(dash.id)
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
            if !pin_visible_to(pool, user_id, &raw, dash.owner_user_id, keyring).await? {
                continue;
            }
            let none = VaultKeyring::default();
            let ring = if raw.is_vault && raw.owner_user_id == user_id {
                keyring
            } else {
                &none
            };
            let Some(view) = bookmark_to_view(pool, raw, ring, user_id).await? else {
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
        shared_with,
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
    keyring: &VaultKeyring,
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

    let title = dash.title.clone();
    let id = dash.id;
    let view = load_dashboard_view(pool, user_id, dash, CategoryAccess::Owner, keyring).await?;
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Created,
        linkdean_core::ActivitySubjectType::Dashboard,
        id,
        &title,
    )
    .await;
    Ok(view)
}

pub async fn put_default_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    req: PutDashboardLayout,
    keyring: &VaultKeyring,
) -> Result<DashboardView> {
    let id = match pinned_shared_dashboard_id(pool, user_id).await? {
        Some(id) => id,
        None => ensure_default_dashboard(pool, user_id).await?.id,
    };
    put_dashboard(pool, user_id, id, req, keyring).await
}

pub async fn put_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: PutDashboardLayout,
    keyring: &VaultKeyring,
) -> Result<DashboardView> {
    let (dash, _) = require_dashboard_write(pool, user_id, id).await?;
    let before_title = dash.title.clone();
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(dash.title.as_str());
    let section_count = req.sections.len();
    let pin_count: usize = req.sections.iter().map(|s| s.bookmark_ids.len()).sum();

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

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

    // The editor only sees some pins: not trashed bookmarks, not locked vault pins, not
    // another member's vault pins. Keep those, since the layout sent cannot mention them.
    let existing: Vec<(i32, String, Uuid)> = sqlx::query_as(
        r#"
        SELECT s.position, s.title, i.bookmark_id
        FROM dashboard_items i
        JOIN dashboard_sections s ON s.id = i.section_id
        WHERE s.dashboard_id = $1
        ORDER BY s.position, s.created_at, i.position, i.created_at
        "#,
    )
    .bind(dash.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let mut hidden: Vec<(usize, String, Uuid)> = Vec::new();
    let mut old_index = std::collections::HashMap::new();
    let mut already_pinned = std::collections::HashSet::new();
    for (pos, sec_title, bookmark_id) in existing {
        let next = old_index.len();
        let index = *old_index.entry(pos).or_insert(next);
        already_pinned.insert(bookmark_id);
        if pin_hidden_from(pool, user_id, bookmark_id, dash.owner_user_id, keyring).await? {
            hidden.push((index, sec_title, bookmark_id));
        }
    }

    sqlx::query("DELETE FROM dashboard_sections WHERE dashboard_id = $1")
        .bind(dash.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let mut new_sections: Vec<(Uuid, std::collections::HashSet<Uuid>, i32)> = Vec::new();

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
            let pinned = already_pinned.contains(bookmark_id);
            if pin_hidden_from(pool, user_id, *bookmark_id, dash.owner_user_id, keyring).await? {
                if pinned {
                    // Kept below from the stored layout, never added from the request.
                    continue;
                }
                return Err(Error::BadRequest(
                    "bookmark not found or not readable".into(),
                ));
            }
            if !seen.insert(*bookmark_id) {
                continue;
            }
            // New pins must be readable by the editor. Pins already on the dashboard, such as
            // the owner's bookmarks a write invitee sees through it, may only be rearranged.
            if !pinned && !bookmark_can_read(pool, user_id, *bookmark_id).await? {
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
        new_sections.push((sec.id, seen, section.bookmark_ids.len() as i32));
    }

    // Hidden pins go back to the section at the same index, else the last one. With no
    // sections left, their old section is recreated so they are not lost.
    for (index, old_title, bookmark_id) in hidden {
        if new_sections.is_empty() {
            let sec_id: Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO dashboard_sections (dashboard_id, title, position)
                VALUES ($1, $2, 0)
                RETURNING id
                "#,
            )
            .bind(dash.id)
            .bind(&old_title)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            new_sections.push((sec_id, std::collections::HashSet::new(), 0));
        }
        let target = index.min(new_sections.len() - 1);
        let (sec_id, seen, next_pos) = &mut new_sections[target];
        if !seen.insert(bookmark_id) {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO dashboard_items (section_id, bookmark_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(*sec_id)
        .bind(bookmark_id)
        .bind(*next_pos)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        *next_pos += 1;
    }

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    if dash.owner_user_id != user_id {
        super::notifications::notify_shared_edit(
            pool,
            user_id,
            dash.owner_user_id,
            linkdean_core::NotificationSubjectType::Dashboard,
            dash.id,
            title,
        )
        .await;
    }

    let mut changes = Vec::new();
    let mut change_keys: Vec<&'static str> = Vec::new();
    if title != before_title {
        changes.push(format!("renamed “{before_title}” → “{title}”"));
        change_keys.push("title");
    }
    changes.push(format!(
        "layout ({section_count} sections, {pin_count} pins)"
    ));
    change_keys.push("layout");
    let summary = super::activity::update_summary(
        linkdean_core::ActivitySubjectType::Dashboard,
        title,
        &changes,
    );
    super::activity::record_activity_with(
        pool,
        user_id,
        linkdean_core::ActivityAction::Updated,
        super::activity::ActivitySubject {
            subject_type: linkdean_core::ActivitySubjectType::Dashboard,
            subject_id: id,
            subject_title: title,
        },
        Some(&summary),
        serde_json::json!({ "changes": change_keys, "sections": section_count, "pins": pin_count }),
    )
    .await;

    get_dashboard(pool, user_id, id, keyring).await
}

async fn pin_dashboard_row(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Dashboard> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

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

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    Ok(dash)
}

pub async fn patch_dashboard(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    req: PatchDashboard,
    keyring: &VaultKeyring,
) -> Result<DashboardView> {
    // Anyone with access may pin, because a pin only changes their own default.
    // A rename still needs write access (same as put layout).
    if req.title.is_some() || req.is_default.is_none() {
        require_dashboard_write(pool, user_id, id).await?;
    }
    let mut pinned_own = false;
    if let Some(pin) = req.is_default {
        let (dash, access) = require_dashboard_read(pool, user_id, id).await?;
        match (access.is_owner(), pin) {
            (true, true) => {
                let _ = pin_dashboard_row(pool, user_id, id).await?;
                clear_shared_dashboard_pin(pool, user_id).await?;
                pinned_own = true;
            }
            (true, false) if dash.is_default => {
                return Err(Error::BadRequest(
                    "pin another dashboard instead of unpinning the only default".into(),
                ));
            }
            (true, false) => {}
            (false, true) => pin_shared_dashboard(pool, user_id, id).await?,
            (false, false) => {
                if pinned_shared_dashboard_id(pool, user_id).await? == Some(id) {
                    clear_shared_dashboard_pin(pool, user_id).await?;
                }
            }
        }
    }

    let mut changes = Vec::new();
    let mut change_keys: Vec<&'static str> = Vec::new();
    let mut activity_title: Option<String> = None;

    if let Some(ref title_raw) = req.title {
        let title = title_raw.trim();
        if title.is_empty() {
            return Err(Error::BadRequest("title required".into()));
        }
        let before = sqlx::query_scalar::<_, String>("SELECT title FROM dashboards WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?
            .unwrap_or_default();
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
        if before != title {
            changes.push(format!("renamed “{before}” → “{title}”"));
            change_keys.push("title");
        }
        activity_title = Some(title.to_string());
    }

    if pinned_own {
        changes.push("pinned as default".into());
        change_keys.push("is_default");
    }

    let view = get_dashboard(pool, user_id, id, keyring).await?;
    if !changes.is_empty() {
        let title = activity_title.as_deref().unwrap_or(view.title.as_str());
        let summary = super::activity::update_summary(
            linkdean_core::ActivitySubjectType::Dashboard,
            title,
            &changes,
        );
        super::activity::record_activity_with(
            pool,
            user_id,
            linkdean_core::ActivityAction::Updated,
            super::activity::ActivitySubject {
                subject_type: linkdean_core::ActivitySubjectType::Dashboard,
                subject_id: id,
                subject_title: title,
            },
            Some(&summary),
            serde_json::json!({ "changes": change_keys }),
        )
        .await;
    }
    Ok(view)
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
    let title = dash.title.clone();

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

    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Deleted,
        linkdean_core::ActivitySubjectType::Dashboard,
        id,
        &title,
    )
    .await;

    list_dashboards(pool, user_id).await
}

pub async fn restore_dashboard(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let title = sqlx::query_scalar::<_, String>(
        r#"
        SELECT title FROM dashboards
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
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
    super::activity::record_activity(
        pool,
        user_id,
        linkdean_core::ActivityAction::Restored,
        linkdean_core::ActivitySubjectType::Dashboard,
        id,
        &title,
    )
    .await;
    Ok(())
}

pub async fn purge_dashboard(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<()> {
    let title = sqlx::query_scalar::<_, String>(
        r#"
        SELECT title FROM dashboards
        WHERE id = $1 AND owner_user_id = $2 AND deleted_at IS NOT NULL
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
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
    super::activity::record_activity_with(
        pool,
        user_id,
        linkdean_core::ActivityAction::Deleted,
        super::activity::ActivitySubject {
            subject_type: linkdean_core::ActivitySubjectType::Dashboard,
            subject_id: id,
            subject_title: &title,
        },
        Some(&format!("Permanently deleted dashboard: {title}")),
        serde_json::json!({ "purged": true }),
    )
    .await;
    Ok(())
}
