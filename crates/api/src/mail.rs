//! Queues rendered email and delivers it in the background.

use crate::state::AppState;
use linkdean_db::{self as db, EmailRetry, NewOutboxEmail};
use linkdean_mail::{render, MailConfig, Mailer, Rendered, Site};
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;

/// How often the sender looks for due email.
const POLL: Duration = Duration::from_secs(5);
/// How many emails one pass claims.
const BATCH: i64 = 20;
/// Longest one send may take. lettre's own timeout applies per SMTP step, so a stalled server
/// could otherwise hold an email past its five-minute claim.
const SEND_TIMEOUT: Duration = Duration::from_secs(60);

pub struct Mail {
    pub cfg: MailConfig,
    pub mailer: Mailer,
    pub site: Site,
}

impl Mail {
    pub fn new(cfg: &MailConfig, app_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            cfg: cfg.clone(),
            mailer: Mailer::new(cfg)?,
            site: Site::new(app_url),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Queued {
    Queued,
    /// Mail is off. Callers that need email fall back to today's behaviour.
    Disabled,
}

/// Render `template` with `data` and queue it for `to`. Never sends inline.
pub async fn enqueue(
    state: &AppState,
    to: &str,
    template: &str,
    data: &Value,
) -> linkdean_core::Result<Queued> {
    let Some(mail) = state.mail.as_deref() else {
        return Ok(Queued::Disabled);
    };
    let r =
        render(template, data, &mail.site).map_err(|e| linkdean_core::Error::msg(e.to_string()))?;
    db::enqueue_email(
        &state.pool,
        &NewOutboxEmail {
            recipient: to.into(),
            template: template.into(),
            subject: r.subject,
            html: r.html,
            text: r.text,
        },
    )
    .await?;
    Ok(Queued::Queued)
}

/// Deliver due email until the process ends. Each tick first turns pending notification emails
/// into outbox rows, so they go out in the same tick. With mail off, each tick discards pending
/// notification emails instead, so turning mail on later sends no backlog.
pub async fn run_sender(state: AppState) {
    let mut tick = tokio::time::interval(POLL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let Some(mail) = state.mail.clone() else {
        loop {
            tick.tick().await;
            crate::notification_mail::discard_notification_emails(&state.pool).await;
        }
    };
    loop {
        tick.tick().await;
        crate::notification_mail::queue_notification_emails(&state).await;
        deliver_due(&state.pool, &mail.mailer).await;
    }
}

/// One pass: claim due email, send each, record the outcome. Returns how many it tried.
pub async fn deliver_due(pool: &PgPool, mailer: &Mailer) -> usize {
    let batch = match db::claim_due_emails(pool, BATCH).await {
        Ok(batch) => batch,
        Err(e) => {
            tracing::warn!(error = %e, "claiming queued email failed");
            return 0;
        }
    };
    for email in &batch {
        // The claim may have lapsed while earlier emails in the batch were sending; another
        // sender could hold this one now.
        match db::renew_email_claim(pool, email.id).await {
            Ok(true) => {}
            Ok(false) => {
                tracing::info!(email_id = %email.id, "email claim lapsed, skipping");
                continue;
            }
            Err(e) => {
                tracing::warn!(email_id = %email.id, error = %e, "renewing email claim failed");
                continue;
            }
        }
        let rendered = Rendered {
            subject: email.subject.clone(),
            html: email.html.clone(),
            text: email.text.clone(),
        };
        let sent = tokio::time::timeout(SEND_TIMEOUT, mailer.send(&email.recipient, &rendered))
            .await
            .unwrap_or_else(|_| Err("SMTP send timed out after 60 s".into()));
        match sent {
            Ok(()) => {
                if let Err(e) = db::mark_email_sent(pool, email.id).await {
                    tracing::warn!(email_id = %email.id, error = %e, "marking email sent failed");
                }
            }
            Err(err) => match db::record_email_failure(pool, email.id, &err).await {
                Ok(EmailRetry::GaveUp) => {
                    tracing::warn!(email_id = %email.id, error = %err, "gave up sending email")
                }
                Ok(EmailRetry::Retrying) => {
                    tracing::info!(email_id = %email.id, attempt = email.attempts + 1, error = %err, "email send failed, will retry")
                }
                Err(e) => {
                    tracing::warn!(email_id = %email.id, error = %e, "recording email failure failed")
                }
            },
        }
    }
    batch.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the throwaway Postgres and a throwaway Mailpit; see crates/mail/tests/smtp.rs.
    #[tokio::test]
    #[ignore = "needs Postgres and a Mailpit container"]
    async fn queued_email_is_delivered_and_marked_sent() {
        let pool = db::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        let port = std::env::var("MAILPIT_SMTP_PORT").unwrap();
        let cfg = MailConfig::from_vars(|k| match k {
            "SMTP_HOST" => Some("127.0.0.1".into()),
            "SMTP_PORT" => Some(port.clone()),
            "SMTP_TLS" => Some("none".into()),
            "MAIL_FROM" => Some("a@example.com".into()),
            _ => None,
        })
        .unwrap()
        .unwrap();
        let mail = Mail::new(&cfg, "http://localhost:5173").unwrap();
        let to = format!("outbox-{}@example.com", uuid::Uuid::new_v4().simple());
        let r = render("test", &serde_json::json!({ "recipient": to }), &mail.site).unwrap();
        let id = db::enqueue_email(
            &pool,
            &NewOutboxEmail {
                recipient: to,
                template: "test".into(),
                subject: r.subject,
                html: r.html,
                text: r.text,
            },
        )
        .await
        .unwrap();
        assert!(deliver_due(&pool, &mail.mailer).await >= 1);
        let (status,): (String,) = sqlx::query_as("SELECT status FROM email_outbox WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "sent");
        sqlx::query("DELETE FROM email_outbox WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
