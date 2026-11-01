use linkdean_core::{
    Error, Notification, NotificationKind, NotificationSubjectType, NotificationView, Result,
    ShareAccess,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::users::{find_user_by_id, share_invitee_label, user_share_label};

#[derive(Debug, sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    #[allow(dead_code)]
    recipient_user_id: Uuid,
    actor_user_id: Option<Uuid>,
    kind: String,
    subject_type: String,
    subject_id: Uuid,
    subject_title: String,
    body: String,
    payload: serde_json::Value,
    read_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    actor_email: Option<String>,
    actor_display_name: Option<String>,
}

fn to_view(row: NotificationRow) -> Result<NotificationView> {
    let kind = NotificationKind::parse(&row.kind).map_err(Error::BadRequest)?;
    let subject_type =
        NotificationSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)?;
    let actor_display_name = match (
        row.actor_display_name.as_deref(),
        row.actor_email.as_deref(),
    ) {
        (Some(_), Some(email)) | (None, Some(email)) => Some(share_invitee_label(
            row.actor_display_name.as_deref(),
            email,
            "Someone",
        )),
        _ => None,
    };
    Ok(NotificationView {
        id: row.id,
        kind,
        subject_type,
        subject_id: row.subject_id,
        subject_title: row.subject_title,
        body: row.body,
        payload: row.payload,
        actor_user_id: row.actor_user_id,
        actor_display_name,
        read_at: row.read_at,
        created_at: row.created_at,
    })
}

async fn insert_notification(
    pool: &PgPool,
    recipient_user_id: Uuid,
    actor_user_id: Option<Uuid>,
    kind: NotificationKind,
    subject_type: NotificationSubjectType,
    subject_id: Uuid,
    subject_title: &str,
    body: &str,
    payload: serde_json::Value,
) -> Result<()> {
    if actor_user_id == Some(recipient_user_id) {
        return Ok(());
    }
    if let Some(actor) = actor_user_id {
        if super::blocks::viewer_hides_user(pool, recipient_user_id, actor).await? {
            return Ok(());
        }
    }
    sqlx::query(
        r#"
        INSERT INTO notifications (
            recipient_user_id, actor_user_id, kind, subject_type, subject_id,
            subject_title, body, payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(recipient_user_id)
    .bind(actor_user_id)
    .bind(kind.as_str())
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(subject_title)
    .bind(body)
    .bind(payload)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Soft-fail wrapper so share/edit paths stay resilient.
pub async fn try_notify(
    pool: &PgPool,
    recipient_user_id: Uuid,
    actor_user_id: Option<Uuid>,
    kind: NotificationKind,
    subject_type: NotificationSubjectType,
    subject_id: Uuid,
    subject_title: &str,
    body: &str,
    payload: serde_json::Value,
) {
    if let Err(e) = insert_notification(
        pool,
        recipient_user_id,
        actor_user_id,
        kind,
        subject_type,
        subject_id,
        subject_title,
        body,
        payload,
    )
    .await
    {
        tracing::warn!(?e, "notification insert failed");
    }
}

pub async fn notify_category_share_invite(
    pool: &PgPool,
    actor_user_id: Uuid,
    invitee_user_id: Uuid,
    category_id: Uuid,
    category_name: &str,
    access: ShareAccess,
) {
    let actor_label = actor_label(pool, actor_user_id).await;
    let access_label = access_label(access);
    try_notify(
        pool,
        invitee_user_id,
        Some(actor_user_id),
        NotificationKind::ShareInvite,
        NotificationSubjectType::Category,
        category_id,
        category_name,
        &format!("{actor_label} shared folder “{category_name}” with you ({access_label})"),
        json!({ "access": access.as_str() }),
    )
    .await;
}

pub async fn notify_category_share_access(
    pool: &PgPool,
    actor_user_id: Uuid,
    invitee_user_id: Uuid,
    category_id: Uuid,
    category_name: &str,
    access: ShareAccess,
) {
    let actor_label = actor_label(pool, actor_user_id).await;
    let access_label = access_label(access);
    try_notify(
        pool,
        invitee_user_id,
        Some(actor_user_id),
        NotificationKind::ShareAccessChanged,
        NotificationSubjectType::Category,
        category_id,
        category_name,
        &format!(
            "{actor_label} changed your access on folder “{category_name}” to {access_label}"
        ),
        json!({ "access": access.as_str() }),
    )
    .await;
}

pub async fn notify_category_share_revoked(
    pool: &PgPool,
    actor_user_id: Uuid,
    invitee_user_id: Uuid,
    category_id: Uuid,
    category_name: &str,
) {
    let actor_label = actor_label(pool, actor_user_id).await;
    try_notify(
        pool,
        invitee_user_id,
        Some(actor_user_id),
        NotificationKind::ShareRevoked,
        NotificationSubjectType::Category,
        category_id,
        category_name,
        &format!("{actor_label} removed your access to folder “{category_name}”"),
        json!({}),
    )
    .await;
}

pub async fn notify_dashboard_share_invite(
    pool: &PgPool,
    actor_user_id: Uuid,
    invitee_user_id: Uuid,
    dashboard_id: Uuid,
    dashboard_title: &str,
    access: ShareAccess,
) {
    let actor_label = actor_label(pool, actor_user_id).await;
    let access_label = access_label(access);
    try_notify(
        pool,
        invitee_user_id,
        Some(actor_user_id),
        NotificationKind::ShareInvite,
        NotificationSubjectType::Dashboard,
        dashboard_id,
        dashboard_title,
        &format!("{actor_label} shared dashboard “{dashboard_title}” with you ({access_label})"),
        json!({ "access": access.as_str() }),
    )
    .await;
}

pub async fn notify_dashboard_share_access(
    pool: &PgPool,
    actor_user_id: Uuid,
    invitee_user_id: Uuid,
    dashboard_id: Uuid,
    dashboard_title: &str,
    access: ShareAccess,
) {
    let actor_label = actor_label(pool, actor_user_id).await;
    let access_label = access_label(access);
    try_notify(
        pool,
        invitee_user_id,
        Some(actor_user_id),
        NotificationKind::ShareAccessChanged,
        NotificationSubjectType::Dashboard,
        dashboard_id,
        dashboard_title,
        &format!(
            "{actor_label} changed your access on dashboard “{dashboard_title}” to {access_label}"
        ),
        json!({ "access": access.as_str() }),
    )
    .await;
}

pub async fn notify_dashboard_share_revoked(
    pool: &PgPool,
    actor_user_id: Uuid,
    invitee_user_id: Uuid,
    dashboard_id: Uuid,
    dashboard_title: &str,
) {
    let actor_label = actor_label(pool, actor_user_id).await;
    try_notify(
        pool,
        invitee_user_id,
        Some(actor_user_id),
        NotificationKind::ShareRevoked,
        NotificationSubjectType::Dashboard,
        dashboard_id,
        dashboard_title,
        &format!("{actor_label} removed your access to dashboard “{dashboard_title}”"),
        json!({}),
    )
    .await;
}

pub async fn notify_shared_edit(
    pool: &PgPool,
    actor_user_id: Uuid,
    owner_user_id: Uuid,
    subject_type: NotificationSubjectType,
    subject_id: Uuid,
    subject_title: &str,
) {
    if actor_user_id == owner_user_id {
        return;
    }
    let actor_label = actor_label(pool, actor_user_id).await;
    let noun = match subject_type {
        NotificationSubjectType::Category => "folder",
        NotificationSubjectType::Dashboard => "dashboard",
        NotificationSubjectType::Bookmark => "bookmark",
        NotificationSubjectType::User => "profile",
    };
    try_notify(
        pool,
        owner_user_id,
        Some(actor_user_id),
        NotificationKind::SharedEdit,
        subject_type,
        subject_id,
        subject_title,
        &format!("{actor_label} edited shared {noun} “{subject_title}”"),
        json!({}),
    )
    .await;
}

async fn actor_label(pool: &PgPool, actor_user_id: Uuid) -> String {
    match find_user_by_id(pool, actor_user_id).await {
        Ok(Some(u)) => user_share_label(&u),
        _ => "Someone".into(),
    }
}

fn access_label(access: ShareAccess) -> &'static str {
    match access {
        ShareAccess::Read => "read-only",
        ShareAccess::Write => "read-write",
    }
}

pub async fn list_notifications(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<NotificationView>> {
    let limit = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT
            n.id, n.recipient_user_id, n.actor_user_id, n.kind, n.subject_type,
            n.subject_id, n.subject_title, n.body, n.payload, n.read_at, n.created_at,
            u.email AS actor_email,
            u.display_name AS actor_display_name
        FROM notifications n
        LEFT JOIN users u ON u.id = n.actor_user_id
        WHERE n.recipient_user_id = $1
          AND (
            n.actor_user_id IS NULL
            OR (
              NOT EXISTS (
                SELECT 1 FROM user_blocks ub
                WHERE (ub.blocker_id = $1 AND ub.blocked_id = n.actor_user_id)
                   OR (ub.blocker_id = n.actor_user_id AND ub.blocked_id = $1)
              )
              AND NOT EXISTS (
                SELECT 1 FROM user_mutes um
                WHERE um.muter_id = $1 AND um.muted_id = n.actor_user_id
              )
            )
          )
        ORDER BY n.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    rows.into_iter().map(to_view).collect()
}

pub async fn unread_notification_count(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let (n,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM notifications n
        WHERE n.recipient_user_id = $1 AND n.read_at IS NULL
          AND (
            n.actor_user_id IS NULL
            OR (
              NOT EXISTS (
                SELECT 1 FROM user_blocks ub
                WHERE (ub.blocker_id = $1 AND ub.blocked_id = n.actor_user_id)
                   OR (ub.blocker_id = n.actor_user_id AND ub.blocked_id = $1)
              )
              AND NOT EXISTS (
                SELECT 1 FROM user_mutes um
                WHERE um.muter_id = $1 AND um.muted_id = n.actor_user_id
              )
            )
          )
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n)
}

pub async fn mark_notification_read(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> Result<NotificationView> {
    let row = sqlx::query_as::<_, NotificationRow>(
        r#"
        WITH updated AS (
            UPDATE notifications
            SET read_at = COALESCE(read_at, now())
            WHERE id = $1 AND recipient_user_id = $2
            RETURNING *
        )
        SELECT
            u.id, u.recipient_user_id, u.actor_user_id, u.kind, u.subject_type,
            u.subject_id, u.subject_title, u.body, u.payload, u.read_at, u.created_at,
            usr.email AS actor_email,
            usr.display_name AS actor_display_name
        FROM updated u
        LEFT JOIN users usr ON usr.id = u.actor_user_id
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;
    to_view(row)
}

pub async fn mark_all_notifications_read(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let res = sqlx::query(
        r#"
        UPDATE notifications
        SET read_at = now()
        WHERE recipient_user_id = $1 AND read_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(res.rows_affected() as i64)
}

/// Test helper / debug: fetch raw row.
#[allow(dead_code)]
pub async fn get_notification(pool: &PgPool, id: Uuid) -> Result<Option<Notification>> {
    sqlx::query_as::<_, Notification>("SELECT * FROM notifications WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}
