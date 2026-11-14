//! Which notifications are also emailed, and the queue the API mailer drains.
//!
//! A new notification gets `email_state = 'pending'` when its recipient's address is verified and
//! their preference for the kind (or the kind's default) is on, `'digest'` when they chose the
//! daily digest for it, otherwise `'none'`. The mailer claims pending rows at once and digest rows
//! once a day; either marks them `'queued'` and hands them to the outbox.

use chrono::{DateTime, Utc};
use linkdean_core::unsubscribe::UnsubscribeScope;
use linkdean_core::{EmailDelivery, Error, NotificationKind, NotificationSubjectType, Result};
use sqlx::{AssertSqlSafe, PgPool};
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
            THEN CASE WHEN EXISTS (
                SELECT 1 FROM user_notification_email_prefs ep
                WHERE ep.user_id = {recipient} AND ep.kind = {kind} AND ep.delivery = 'daily'
            ) THEN 'digest' ELSE 'pending' END
            ELSE 'none' END"
    )
}

/// One kind's email setting, defaults filled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationEmailPref {
    pub kind: NotificationKind,
    pub enabled: bool,
    pub delivery: EmailDelivery,
}

/// Every kind with whether and how the user gets it by email, in settings order.
pub async fn get_notification_email_prefs(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<NotificationEmailPref>> {
    let rows: Vec<(String, bool, String)> = sqlx::query_as(
        "SELECT kind, enabled, delivery FROM user_notification_email_prefs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    Ok(NotificationKind::ALL
        .iter()
        .map(|&kind| {
            let row = rows.iter().find(|(k, _, _)| k == kind.as_str());
            NotificationEmailPref {
                kind,
                enabled: row.map_or(kind.email_by_default(), |(_, on, _)| *on),
                delivery: row
                    .and_then(|(_, _, d)| EmailDelivery::parse(d).ok())
                    .unwrap_or_default(),
            }
        })
        .collect())
}

/// Saves one kind's setting and moves its waiting emails to match: off drops them, instant sends
/// the ones waiting for the digest, daily holds back the ones not yet sent. A kind that cannot
/// wait for the digest is refused `Daily`.
pub async fn set_notification_email_pref(
    pool: &PgPool,
    user_id: Uuid,
    kind: NotificationKind,
    enabled: bool,
    delivery: EmailDelivery,
) -> Result<()> {
    if delivery == EmailDelivery::Daily && !kind.allows_digest() {
        return Err(Error::BadRequest(format!(
            "{} cannot wait for the daily digest",
            kind.label()
        )));
    }
    let mut tx = pool.begin().await.map_err(db_err)?;
    sqlx::query(
        r#"
        INSERT INTO user_notification_email_prefs (user_id, kind, enabled, delivery)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, kind)
        DO UPDATE SET enabled = EXCLUDED.enabled, delivery = EXCLUDED.delivery
        "#,
    )
    .bind(user_id)
    .bind(kind.as_str())
    .bind(enabled)
    .bind(delivery.as_str())
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    let target = match (enabled, delivery) {
        (false, _) => "none",
        (true, EmailDelivery::Instant) => "pending",
        (true, EmailDelivery::Daily) => "digest",
    };
    sqlx::query(
        r#"
        UPDATE notifications SET email_state = $3
        WHERE recipient_user_id = $1 AND kind = $2 AND email_state IN ('pending', 'digest')
        "#,
    )
    .bind(user_id)
    .bind(kind.as_str())
    .bind(target)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(())
}

/// What an unsubscribe link turned off, for its confirmation page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribed {
    /// The kinds now off. Empty when the digest scope found none on.
    pub kinds: Vec<NotificationKind>,
}

/// Turns off the email an unsubscribe link names and drops its waiting emails. A kind stays off
/// until the user turns it on again; the digest scope turns off every kind sent as a digest.
/// `NotFound` when the user is gone.
pub async fn unsubscribe(
    pool: &PgPool,
    user_id: Uuid,
    scope: UnsubscribeScope,
) -> Result<Unsubscribed> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
    if exists.is_none() {
        return Err(Error::NotFound);
    }
    let kinds: Vec<(String,)> = match scope {
        UnsubscribeScope::Kind(kind) => sqlx::query_as(
            r#"
            INSERT INTO user_notification_email_prefs (user_id, kind, enabled)
            VALUES ($1, $2, false)
            ON CONFLICT (user_id, kind) DO UPDATE SET enabled = false
            RETURNING kind
            "#,
        )
        .bind(user_id)
        .bind(kind.as_str())
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?,
        UnsubscribeScope::Digest => sqlx::query_as(
            r#"
            UPDATE user_notification_email_prefs SET enabled = false
            WHERE user_id = $1 AND delivery = 'daily' AND enabled
            RETURNING kind
            "#,
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?,
    };
    let names: Vec<String> = kinds.into_iter().map(|(k,)| k).collect();
    sqlx::query(
        r#"
        UPDATE notifications SET email_state = 'none'
        WHERE recipient_user_id = $1 AND kind = ANY($2) AND email_state IN ('pending', 'digest')
        "#,
    )
    .bind(user_id)
    .bind(&names)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(Unsubscribed {
        kinds: names
            .iter()
            .filter_map(|k| NotificationKind::parse(k).ok())
            .collect(),
    })
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

/// The columns a claimed row returns, for the CTE named `claimed`.
const CLAIMED_COLUMNS: &str =
    "id, recipient_user_id, actor_user_id, kind, subject_type, subject_id,
    subject_title, body, payload, created_at";

/// Selects what each row in `claimed` needs for its email, dropping rows whose recipient's address
/// is no longer verified, or who has since blocked or muted the actor (or been blocked by them).
const CLAIMED_SELECT: &str = r#"
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
        ORDER BY c.recipient_user_id, c.created_at
"#;

fn into_pending(rows: Vec<PendingRow>) -> Vec<PendingNotificationEmail> {
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
    out
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
    let sql = format!(
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
            RETURNING {CLAIMED_COLUMNS}
        )
        {CLAIMED_SELECT}
        "#
    );
    let rows = sqlx::query_as::<_, PendingRow>(AssertSqlSafe(sql))
        .bind(limit.max(1))
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
    let mut out = into_pending(rows);
    out.sort_by_key(|n| n.created_at);
    Ok(out)
}

/// One recipient's daily digest: their notifications waiting for it, oldest first.
#[derive(Debug, Clone)]
pub struct NotificationDigest {
    pub recipient_user_id: Uuid,
    pub recipient_email: String,
    pub items: Vec<PendingNotificationEmail>,
}

/// Claims the digests that are due, for up to `limit_users` recipients, and marks their rows
/// `queued`.
///
/// A digest is due once the recipient's local time (their profile time zone, else UTC) has passed
/// `local_hour` today and none went out since then, so each user gets at most one a day. One
/// transaction under an advisory lock, so concurrent senders never split a digest. Rows dropped by
/// the usual checks (unverified address, blocks, mutes) are not returned, and a recipient left
/// with none gets no email.
pub async fn claim_due_notification_digests(
    pool: &PgPool,
    local_hour: u32,
    limit_users: i64,
) -> Result<Vec<NotificationDigest>> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('linkdean_notification_digests'))")
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let due: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        WITH waiting AS (
            SELECT DISTINCT recipient_user_id AS user_id FROM notifications
            WHERE email_state = 'digest'
        ),
        zoned AS (
            SELECT w.user_id,
                CASE WHEN u.timezone IN (SELECT name FROM pg_timezone_names)
                     THEN u.timezone ELSE 'UTC' END AS tz
            FROM waiting w JOIN users u ON u.id = w.user_id
        ),
        slotted AS (
            SELECT z.user_id,
                (date_trunc('day', now() AT TIME ZONE z.tz) + make_interval(hours => $1))
                    AT TIME ZONE z.tz AS slot
            FROM zoned z
        )
        SELECT s.user_id FROM slotted s
        LEFT JOIN notification_digest_runs dr ON dr.user_id = s.user_id
        WHERE now() >= s.slot AND (dr.last_sent_at IS NULL OR dr.last_sent_at < s.slot)
        ORDER BY s.user_id
        LIMIT $2
        "#,
    )
    .bind(local_hour.min(23) as i32)
    .bind(limit_users.max(1))
    .fetch_all(&mut *tx)
    .await
    .map_err(db_err)?;
    let due: Vec<Uuid> = due.into_iter().map(|(id,)| id).collect();
    if due.is_empty() {
        tx.commit().await.map_err(db_err)?;
        return Ok(Vec::new());
    }
    let sql = format!(
        r#"
        WITH claimed AS (
            UPDATE notifications SET email_state = 'queued'
            WHERE email_state = 'digest' AND recipient_user_id = ANY($1)
            RETURNING {CLAIMED_COLUMNS}
        )
        {CLAIMED_SELECT}
        "#
    );
    let rows = sqlx::query_as::<_, PendingRow>(AssertSqlSafe(sql))
        .bind(&due)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
    sqlx::query(
        r#"
        INSERT INTO notification_digest_runs (user_id, last_sent_at)
        SELECT unnest($1::uuid[]), now()
        ON CONFLICT (user_id) DO UPDATE SET last_sent_at = EXCLUDED.last_sent_at
        "#,
    )
    .bind(&due)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;

    let mut digests: Vec<NotificationDigest> = Vec::new();
    for item in into_pending(rows) {
        match digests.last_mut() {
            Some(d) if d.recipient_user_id == item.recipient_user_id => d.items.push(item),
            _ => digests.push(NotificationDigest {
                recipient_user_id: item.recipient_user_id,
                recipient_email: item.recipient_email.clone(),
                items: vec![item],
            }),
        }
    }
    Ok(digests)
}

/// Marks up to `limit` pending or digest notification emails `none` without sending them, for when
/// mail is off, so turning mail on later does not send a backlog. Returns how many were discarded.
pub async fn discard_pending_notification_emails(pool: &PgPool, limit: i64) -> Result<u64> {
    let res = sqlx::query(
        r#"
        UPDATE notifications SET email_state = 'none'
        WHERE id IN (
            SELECT id FROM notifications
            WHERE email_state IN ('pending', 'digest')
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

/// The database's clock, for comparing with `created_at` of rows inserted after this call.
pub async fn database_now(pool: &PgPool) -> Result<DateTime<Utc>> {
    let (now,): (DateTime<Utc>,) = sqlx::query_as("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    Ok(now)
}

/// The share invite notification `actor_user_id` created for this subject at or after `since`,
/// when it is to be emailed (pending or already queued). Lets the share routes tell whether the
/// share they just made will send an invite email.
pub async fn find_new_share_invite_email(
    pool: &PgPool,
    actor_user_id: Uuid,
    subject_type: NotificationSubjectType,
    subject_id: Uuid,
    since: DateTime<Utc>,
) -> Result<Option<Uuid>> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id FROM notifications
        WHERE actor_user_id = $1 AND kind = $2 AND subject_type = $3 AND subject_id = $4
          AND created_at >= $5 AND email_state IN ('pending', 'queued')
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(actor_user_id)
    .bind(NotificationKind::ShareInvite.as_str())
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(since)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;
    Ok(row.map(|(id,)| id))
}

/// Stops a notification from being emailed. False when the mailer already claimed it.
pub async fn cancel_notification_email(pool: &PgPool, id: Uuid) -> Result<bool> {
    let res = sqlx::query(
        "UPDATE notifications SET email_state = 'none' WHERE id = $1 AND email_state = 'pending'",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(res.rows_affected() == 1)
}
