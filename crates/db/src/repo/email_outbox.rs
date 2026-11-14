//! The email outbox. Request handlers enqueue; the API's sender claims, sends and settles.

use chrono::{DateTime, Utc};
use linkdean_core::{Error, Result};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub struct NewOutboxEmail {
    pub recipient: String,
    pub template: String,
    pub subject: String,
    pub html: String,
    pub text: String,
    /// The one-click unsubscribe URL for the `List-Unsubscribe` header, when the email has one.
    pub list_unsubscribe: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OutboxEmail {
    pub id: Uuid,
    pub recipient: String,
    pub subject: String,
    pub html: String,
    pub text: String,
    pub list_unsubscribe: Option<String>,
    pub attempts: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailRetry {
    Retrying,
    GaveUp,
}

/// Sent and failed counts and the latest error cover the last 24 hours; queued is everything
/// still waiting.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EmailStats {
    pub sent: i64,
    pub queued: i64,
    pub failed: i64,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
}

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

pub async fn enqueue_email(pool: &PgPool, email: &NewOutboxEmail) -> Result<Uuid> {
    let (id,): (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO email_outbox (recipient, template, subject, html, text, list_unsubscribe)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(&email.recipient)
    .bind(&email.template)
    .bind(&email.subject)
    .bind(&email.html)
    .bind(&email.text)
    .bind(&email.list_unsubscribe)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    Ok(id)
}

/// Hand out up to `limit` due emails, each to one caller, for five minutes.
pub async fn claim_due_emails(pool: &PgPool, limit: i64) -> Result<Vec<OutboxEmail>> {
    sqlx::query_as::<_, OutboxEmail>(
        r#"
        UPDATE email_outbox
        SET claimed_until = now() + interval '5 minutes', updated_at = now()
        WHERE id IN (
            SELECT id FROM email_outbox
            WHERE status = 'queued'
              AND next_attempt_at <= now()
              AND (claimed_until IS NULL OR claimed_until < now())
            ORDER BY next_attempt_at
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, recipient, subject, html, text, list_unsubscribe, attempts
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

/// Extend this sender's claim on `id` by five minutes before it tries the email. `false` when
/// the claim has lapsed (another sender may hold it now) or the email is no longer queued.
pub async fn renew_email_claim(pool: &PgPool, id: Uuid) -> Result<bool> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        UPDATE email_outbox
        SET claimed_until = now() + interval '5 minutes', updated_at = now()
        WHERE id = $1 AND status = 'queued' AND claimed_until > now()
        RETURNING id
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;
    Ok(row.is_some())
}

/// Mark a claimed email sent. Does nothing when the row is not claimed.
pub async fn mark_email_sent(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE email_outbox
        SET status = 'sent', attempts = attempts + 1, claimed_until = NULL,
            last_error = NULL, updated_at = now()
        WHERE id = $1 AND claimed_until IS NOT NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Schedule the next try (30 s, doubling, at most an hour apart), or give up once the email
/// has waited a day. `NotFound` when the row is not claimed.
pub async fn record_email_failure(pool: &PgPool, id: Uuid, error: &str) -> Result<EmailRetry> {
    let (status,): (String,) = sqlx::query_as(
        r#"
        UPDATE email_outbox
        SET attempts = attempts + 1,
            last_error = $2,
            claimed_until = NULL,
            updated_at = now(),
            status = CASE WHEN created_at + interval '24 hours' <= now() THEN 'failed' ELSE 'queued' END,
            next_attempt_at = now() + LEAST(
                make_interval(secs => 30 * power(2, LEAST(attempts, 10))),
                interval '1 hour'
            )
        WHERE id = $1 AND claimed_until IS NOT NULL
        RETURNING status
        "#,
    )
    .bind(id)
    .bind(error)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?
    .ok_or(Error::NotFound)?;
    Ok(if status == "failed" {
        EmailRetry::GaveUp
    } else {
        EmailRetry::Retrying
    })
}

pub async fn email_stats(pool: &PgPool) -> Result<EmailStats> {
    sqlx::query_as::<_, EmailStats>(
        r#"
        SELECT
            count(*) FILTER (WHERE o.status = 'sent' AND o.updated_at > now() - interval '24 hours') AS sent,
            count(*) FILTER (WHERE o.status = 'queued') AS queued,
            count(*) FILTER (WHERE o.status = 'failed' AND o.updated_at > now() - interval '24 hours') AS failed,
            latest.last_error,
            latest.updated_at AS last_error_at
        FROM email_outbox o
        LEFT JOIN LATERAL (
            SELECT last_error, updated_at FROM email_outbox
            WHERE last_error IS NOT NULL AND updated_at > now() - interval '24 hours'
            ORDER BY updated_at DESC
            LIMIT 1
        ) latest ON TRUE
        GROUP BY latest.last_error, latest.updated_at
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(db_err)
    .map(|row| {
        row.unwrap_or(EmailStats {
            sent: 0,
            queued: 0,
            failed: 0,
            last_error: None,
            last_error_at: None,
        })
    })
}

/// Rows hold rendered bodies, which can carry one-time links, so they are not kept long.
pub async fn purge_old_emails(pool: &PgPool) -> Result<u64> {
    let done = sqlx::query(
        r#"
        DELETE FROM email_outbox
        WHERE (status = 'sent' AND updated_at < now() - interval '7 days')
           OR (status = 'failed' AND updated_at < now() - interval '30 days')
        "#,
    )
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(done.rows_affected())
}
