use linkdean_core::{
    CategoryAccess, CreateDashboardShare, Dashboard, DashboardShare, DashboardShareView, Error,
    Result, ShareAccess, UpdateDashboardShare,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::users::find_user_by_email;

#[derive(Debug, sqlx::FromRow)]
struct DashShareRow {
    id: Uuid,
    dashboard_id: Uuid,
    invitee_user_id: Uuid,
    access: String,
    invited_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    invitee_email: String,
    invitee_display_name: Option<String>,
}

fn to_view(row: DashShareRow) -> Result<DashboardShareView> {
    let access = ShareAccess::parse(&row.access).map_err(Error::BadRequest)?;
    Ok(DashboardShareView {
        id: row.id,
        dashboard_id: row.dashboard_id,
        invitee_user_id: row.invitee_user_id,
        invitee_email: row.invitee_email,
        invitee_display_name: row.invitee_display_name,
        access,
        invited_by_user_id: row.invited_by_user_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn get_dashboard_row(pool: &PgPool, id: Uuid) -> Result<Dashboard> {
    sqlx::query_as::<_, Dashboard>("SELECT * FROM dashboards WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

async fn find_direct_dashboard_share(
    pool: &PgPool,
    dashboard_id: Uuid,
    invitee_user_id: Uuid,
) -> Result<Option<DashboardShare>> {
    sqlx::query_as::<_, DashboardShare>(
        r#"
        SELECT id, dashboard_id, invitee_user_id, access, invited_by_user_id, created_at, updated_at
        FROM dashboard_shares
        WHERE dashboard_id = $1 AND invitee_user_id = $2
        "#,
    )
    .bind(dashboard_id)
    .bind(invitee_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

/// Owner wins. Else direct share. Else NotFound.
// Known issue: Invitees keep access to a dashboard in Trash and can view and edit it. Fix: NotFound
// for non-owners when `deleted_at` is set.
pub async fn dashboard_access(
    pool: &PgPool,
    user_id: Uuid,
    dashboard_id: Uuid,
) -> Result<CategoryAccess> {
    let dash = get_dashboard_row(pool, dashboard_id).await?;
    if dash.owner_user_id == user_id {
        return Ok(CategoryAccess::Owner);
    }
    if let Some(share) = find_direct_dashboard_share(pool, dashboard_id, user_id).await? {
        let access = ShareAccess::parse(&share.access).map_err(Error::BadRequest)?;
        return Ok(access.to_dashboard_access());
    }
    Err(Error::NotFound)
}

pub async fn require_dashboard_read(
    pool: &PgPool,
    user_id: Uuid,
    dashboard_id: Uuid,
) -> Result<(Dashboard, CategoryAccess)> {
    let access = dashboard_access(pool, user_id, dashboard_id).await?;
    let dash = get_dashboard_row(pool, dashboard_id).await?;
    Ok((dash, access))
}

pub async fn require_dashboard_write(
    pool: &PgPool,
    user_id: Uuid,
    dashboard_id: Uuid,
) -> Result<(Dashboard, CategoryAccess)> {
    let (dash, access) = require_dashboard_read(pool, user_id, dashboard_id).await?;
    if !access.can_write() {
        return Err(Error::Forbidden);
    }
    Ok((dash, access))
}

pub async fn require_dashboard_owner(
    pool: &PgPool,
    user_id: Uuid,
    dashboard_id: Uuid,
) -> Result<Dashboard> {
    let (dash, access) = require_dashboard_read(pool, user_id, dashboard_id).await?;
    if !access.is_owner() {
        return Err(Error::Forbidden);
    }
    Ok(dash)
}

/// The shared dashboard this user pinned as their default, if it still exists.
pub async fn pinned_shared_dashboard_id(pool: &PgPool, user_id: Uuid) -> Result<Option<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT d.id
        FROM dashboard_shares ds
        JOIN dashboards d ON d.id = ds.dashboard_id
        WHERE ds.invitee_user_id = $1
          AND ds.is_pinned
          AND d.deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

/// Pin a dashboard shared with this user. Their own default stays set as the fallback.
pub async fn pin_shared_dashboard(pool: &PgPool, user_id: Uuid, dashboard_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE dashboard_shares
        SET is_pinned = FALSE, updated_at = now()
        WHERE invitee_user_id = $1 AND is_pinned
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let res = sqlx::query(
        r#"
        UPDATE dashboard_shares
        SET is_pinned = TRUE, updated_at = now()
        WHERE invitee_user_id = $1 AND dashboard_id = $2
        "#,
    )
    .bind(user_id)
    .bind(dashboard_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn clear_shared_dashboard_pin(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE dashboard_shares
        SET is_pinned = FALSE, updated_at = now()
        WHERE invitee_user_id = $1 AND is_pinned
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn list_shared_dashboards(
    pool: &PgPool,
    invitee_user_id: Uuid,
) -> Result<Vec<(Dashboard, CategoryAccess)>> {
    #[derive(Debug, sqlx::FromRow)]
    struct Row {
        id: Uuid,
        owner_user_id: Uuid,
        title: String,
        is_default: bool,
        deleted_at: Option<chrono::DateTime<chrono::Utc>>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        share_access: String,
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            d.id, d.owner_user_id, d.title, d.is_default, d.deleted_at, d.created_at, d.updated_at,
            ds.access AS share_access
        FROM dashboard_shares ds
        JOIN dashboards d ON d.id = ds.dashboard_id
        WHERE ds.invitee_user_id = $1
          AND d.deleted_at IS NULL
        ORDER BY d.title ASC, d.created_at ASC
        "#,
    )
    .bind(invitee_user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let access = ShareAccess::parse(&row.share_access)
            .map_err(Error::BadRequest)?
            .to_dashboard_access();
        out.push((
            Dashboard {
                id: row.id,
                owner_user_id: row.owner_user_id,
                title: row.title,
                is_default: row.is_default,
                deleted_at: row.deleted_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            access,
        ));
    }
    Ok(out)
}

pub async fn dashboard_shared_with(pool: &PgPool, dashboard_id: Uuid) -> Result<Vec<String>> {
    let rows: Vec<(Option<String>, String)> = sqlx::query_as(
        r#"
        SELECT u.display_name, u.email
        FROM dashboard_shares ds
        JOIN users u ON u.id = ds.invitee_user_id
        WHERE ds.dashboard_id = $1
        ORDER BY u.email ASC
        "#,
    )
    .bind(dashboard_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut labels = Vec::new();
    for (display_name, email) in rows {
        let label = super::users::share_invitee_label(display_name.as_deref(), &email, "User");
        if !labels.iter().any(|s| s == &label) {
            labels.push(label);
        }
    }
    Ok(labels)
}

pub async fn dashboard_shared_out(pool: &PgPool, dashboard_id: Uuid) -> Result<bool> {
    Ok(!dashboard_shared_with(pool, dashboard_id).await?.is_empty())
}

pub async fn list_shares_for_dashboard(
    pool: &PgPool,
    owner_user_id: Uuid,
    dashboard_id: Uuid,
) -> Result<Vec<DashboardShareView>> {
    require_dashboard_owner(pool, owner_user_id, dashboard_id).await?;
    let rows = sqlx::query_as::<_, DashShareRow>(
        r#"
        SELECT
            ds.id, ds.dashboard_id, ds.invitee_user_id, ds.access, ds.invited_by_user_id,
            ds.created_at, ds.updated_at,
            u.email AS invitee_email, u.display_name AS invitee_display_name
        FROM dashboard_shares ds
        JOIN users u ON u.id = ds.invitee_user_id
        WHERE ds.dashboard_id = $1
        ORDER BY u.email ASC
        "#,
    )
    .bind(dashboard_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter().map(to_view).collect()
}

pub async fn create_dashboard_share(
    pool: &PgPool,
    owner_user_id: Uuid,
    dashboard_id: Uuid,
    req: CreateDashboardShare,
) -> Result<linkdean_core::CreateDashboardShareOutcome> {
    use linkdean_core::{CreateDashboardShareOutcome, PendingShareSubjectType};

    let dash = require_dashboard_owner(pool, owner_user_id, dashboard_id).await?;
    if super::dashboard_sections::dashboard_has_self_filling_section(pool, dashboard_id).await? {
        return Err(Error::BadRequest(
            "remove the sections that fill themselves before sharing this dashboard".into(),
        ));
    }

    let email = req.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("valid email required".into()));
    }
    let Some(invitee) = find_user_by_email(pool, &email).await? else {
        let invite = super::pending_shares::upsert_pending_share_invite(
            pool,
            owner_user_id,
            PendingShareSubjectType::Dashboard,
            dashboard_id,
            &email,
            req.access,
            None,
        )
        .await?
        .0;
        return Ok(CreateDashboardShareOutcome::Pending { invite });
    };
    if invitee.id == owner_user_id {
        return Err(Error::BadRequest("cannot share with yourself".into()));
    }
    super::blocks::assert_not_blocked(pool, owner_user_id, invitee.id).await?;

    let existed = find_direct_dashboard_share(pool, dashboard_id, invitee.id)
        .await?
        .is_some();
    if !existed {
        // Until the account proves it controls the address, the invite waits for the address
        // like one to an unknown address, and attaches when the account verifies.
        let invitee_user_id = invitee.email_verified_at.is_some().then_some(invitee.id);
        let (invite, created) = super::pending_shares::upsert_pending_share_invite(
            pool,
            owner_user_id,
            PendingShareSubjectType::Dashboard,
            dashboard_id,
            &email,
            req.access,
            invitee_user_id,
        )
        .await?;
        if created && invitee_user_id.is_some() {
            super::notifications::notify_dashboard_share_invite(
                pool,
                owner_user_id,
                invitee.id,
                dashboard_id,
                &dash.title,
                invite.access,
            )
            .await;
        }
        return Ok(CreateDashboardShareOutcome::Pending { invite });
    }
    let access = req.access.as_str();
    let row = sqlx::query_as::<_, DashShareRow>(
        r#"
        INSERT INTO dashboard_shares (dashboard_id, invitee_user_id, access, invited_by_user_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (dashboard_id, invitee_user_id) DO UPDATE
          SET access = EXCLUDED.access, updated_at = now()
        RETURNING
            id, dashboard_id, invitee_user_id, access, invited_by_user_id, created_at, updated_at,
            (SELECT email FROM users WHERE id = invitee_user_id) AS invitee_email,
            (SELECT display_name FROM users WHERE id = invitee_user_id) AS invitee_display_name
        "#,
    )
    .bind(dashboard_id)
    .bind(invitee.id)
    .bind(access)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let view = to_view(row)?;
    super::notifications::notify_dashboard_share_access(
        pool,
        owner_user_id,
        invitee.id,
        dashboard_id,
        &dash.title,
        view.access,
    )
    .await;
    Ok(CreateDashboardShareOutcome::Active { share: view })
}

pub async fn update_dashboard_share(
    pool: &PgPool,
    owner_user_id: Uuid,
    dashboard_id: Uuid,
    invitee_user_id: Uuid,
    req: UpdateDashboardShare,
) -> Result<DashboardShareView> {
    let dash = require_dashboard_owner(pool, owner_user_id, dashboard_id).await?;
    let access = req.access.as_str();
    let row = sqlx::query_as::<_, DashShareRow>(
        r#"
        UPDATE dashboard_shares
        SET access = $3, updated_at = now()
        WHERE dashboard_id = $1 AND invitee_user_id = $2
        RETURNING
            id, dashboard_id, invitee_user_id, access, invited_by_user_id, created_at, updated_at,
            (SELECT email FROM users WHERE id = invitee_user_id) AS invitee_email,
            (SELECT display_name FROM users WHERE id = invitee_user_id) AS invitee_display_name
        "#,
    )
    .bind(dashboard_id)
    .bind(invitee_user_id)
    .bind(access)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    let view = to_view(row)?;
    super::notifications::notify_dashboard_share_access(
        pool,
        owner_user_id,
        invitee_user_id,
        dashboard_id,
        &dash.title,
        view.access,
    )
    .await;
    Ok(view)
}

pub async fn delete_dashboard_share(
    pool: &PgPool,
    owner_user_id: Uuid,
    dashboard_id: Uuid,
    invitee_user_id: Uuid,
) -> Result<()> {
    let dash = require_dashboard_owner(pool, owner_user_id, dashboard_id).await?;
    let res = sqlx::query(
        "DELETE FROM dashboard_shares WHERE dashboard_id = $1 AND invitee_user_id = $2",
    )
    .bind(dashboard_id)
    .bind(invitee_user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    super::notifications::notify_dashboard_share_revoked(
        pool,
        owner_user_id,
        invitee_user_id,
        dashboard_id,
        &dash.title,
    )
    .await;
    Ok(())
}
