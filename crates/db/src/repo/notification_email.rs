//! Which notifications are also emailed, and the queue the API mailer drains.
//!
//! A new notification gets `email_state = 'pending'` when its recipient's address is verified and
//! their preference for the kind (or the kind's default) is on; otherwise `'none'`. The mailer
//! claims pending rows, which marks them `'queued'`, and hands them to the outbox.

use chrono::{DateTime, Utc};
use linkdean_core::{Error, NotificationKind, NotificationSubjectType, Result};
use sqlx::PgPool;
use uuid::Uuid;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

/// SQL for a new notification's `email_state`. Arguments are placeholders or literals: the
/// recipient's id, the kind's text and the kind's default (a boolean).
pub(super) fn email_state_sql(recipient: &str, kind: &str, default: &str) -> String {
    format!(
        "CASE WHEN EXISTS (
                SELECT 1 FROM users eu WHERE eu.id = {recipient} AND eu.email_verified_at IS NOT NULL
            ) AND COALESCE((
                SELECT ep.enabled FROM user_notification_email_prefs ep
                WHERE ep.user_id = {recipient} AND ep.kind = {kind}
            ), {default})
            THEN 'pending' ELSE 'none' END"
    )
}

/// Every kind with whether the user gets it by email, defaults filled in.
pub async fn get_notification_email_prefs(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(NotificationKind, bool)>> {
    let rows: Vec<(String, bool)> = sqlx::query_as(
        "SELECT kind, enabled FROM user_notification_email_prefs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    Ok(NotificationKind::ALL
        .iter()
        .map(|&kind| {
            let enabled = rows
                .iter()
                .find(|(k, _)| k == kind.as_str())
                .map_or(kind.email_by_default(), |(_, on)| *on);
            (kind, enabled)
        })
        .collect())
}

pub async fn set_notification_email_pref(
    pool: &PgPool,
    user_id: Uuid,
    kind: NotificationKind,
    enabled: bool,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_notification_email_prefs (user_id, kind, enabled)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, kind) DO UPDATE SET enabled = EXCLUDED.enabled
        "#,
    )
    .bind(user_id)
    .bind(kind.as_str())
    .bind(enabled)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// A notification waiting to be emailed, with what the email needs.
#[derive(Debug, Clone)]
pub struct PendingNotificationEmail {
    pub id: Uuid,
    pub recipient_user_id: Uuid,
    pub recipient_email: String,
    pub actor_user_id: Option<Uuid>,
    /// The actor's display name, else their address; `None` when there is no actor.
    pub actor_label: Option<String>,
    pub kind: NotificationKind,
    pub subject_type: NotificationSubjectType,
    pub subject_id: Uuid,
    pub subject_title: String,
    /// The sentence the in-app notification shows.
    pub body: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct PendingRow {
    id: Uuid,
    recipient_user_id: Uuid,
    recipient_email: String,
    actor_user_id: Option<Uuid>,
    actor_email: Option<String>,
    actor_display_name: Option<String>,
    kind: String,
    subject_type: String,
    subject_id: Uuid,
    subject_title: String,
    body: String,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

/// Claims up to `limit` pending notification emails, oldest first, and marks them `queued`.
///
/// Each row is returned at most once, even to concurrent callers. A claimed row whose recipient's
/// address is no longer verified, or who has since blocked or muted the actor (or been blocked by
/// them), is marked `queued` but not returned: it is dropped, not emailed.
pub async fn claim_pending_notification_emails(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PendingNotificationEmail>> {
    let rows = sqlx::query_as::<_, PendingRow>(
        r#"
        WITH claimed AS (
            UPDATE notifications SET email_state = 'queued'
            WHERE id IN (
                SELECT id FROM notifications
                WHERE email_state = 'pending'
                ORDER BY created_at
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, recipient_user_id, actor_user_id, kind, subject_type, subject_id,
                subject_title, body, payload, created_at
        )
        SELECT
            c.id, c.recipient_user_id, r.email AS recipient_email, c.actor_user_id,
            a.email AS actor_email, a.display_name AS actor_display_name,
            c.kind, c.subject_type, c.subject_id, c.subject_title, c.body, c.payload, c.created_at
        FROM claimed c
        JOIN users r ON r.id = c.recipient_user_id
        LEFT JOIN users a ON a.id = c.actor_user_id
        WHERE r.email_verified_at IS NOT NULL
          AND (
            c.actor_user_id IS NULL
            OR (
              NOT EXISTS (
                SELECT 1 FROM user_blocks ub
                WHERE (ub.blocker_id = c.recipient_user_id AND ub.blocked_id = c.actor_user_id)
                   OR (ub.blocker_id = c.actor_user_id AND ub.blocked_id = c.recipient_user_id)
              )
              AND NOT EXISTS (
                SELECT 1 FROM user_mutes um
                WHERE um.muter_id = c.recipient_user_id AND um.muted_id = c.actor_user_id
              )
            )
          )
        ORDER BY c.created_at
        "#,
    )
    .bind(limit.max(1))
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let (Ok(kind), Ok(subject_type)) = (
            NotificationKind::parse(&row.kind),
            NotificationSubjectType::parse(&row.subject_type),
        ) else {
            tracing::warn!(id = %row.id, kind = row.kind, "skipping notification email of unknown shape");
            continue;
        };
        let actor_label = row.actor_email.as_deref().map(|email| {
            row.actor_display_name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .unwrap_or(email)
                .to_string()
        });
        out.push(PendingNotificationEmail {
            id: row.id,
            recipient_user_id: row.recipient_user_id,
            recipient_email: row.recipient_email,
            actor_user_id: row.actor_user_id,
            actor_label,
            kind,
            subject_type,
            subject_id: row.subject_id,
            subject_title: row.subject_title,
            body: row.body,
            payload: row.payload,
            created_at: row.created_at,
        });
    }
    Ok(out)
}

/// Marks up to `limit` pending notification emails `none` without sending them, for when mail is
/// off, so turning mail on later does not send a backlog. Returns how many were discarded.
pub async fn discard_pending_notification_emails(pool: &PgPool, limit: i64) -> Result<u64> {
    let res = sqlx::query(
        r#"
        UPDATE notifications SET email_state = 'none'
        WHERE id IN (
            SELECT id FROM notifications
            WHERE email_state = 'pending'
            ORDER BY created_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        "#,
    )
    .bind(limit.max(1))
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(res.rows_affected())
}
