use linkdean_core::{
    ActivityAction, ActivityEvent, ActivitySubjectType, Error, ListActivityFilter, Result,
};
use serde_json::json;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct ActivityRow {
    id: Uuid,
    #[allow(dead_code)]
    user_id: Uuid,
    action: String,
    subject_type: String,
    subject_id: Uuid,
    subject_title: String,
    summary: String,
    payload: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

fn to_event(row: ActivityRow) -> Result<ActivityEvent> {
    let action = ActivityAction::parse(&row.action).map_err(Error::BadRequest)?;
    let subject_type = ActivitySubjectType::parse(&row.subject_type).map_err(Error::BadRequest)?;
    Ok(ActivityEvent {
        id: row.id,
        action,
        subject_type,
        subject_id: row.subject_id,
        subject_title: row.subject_title,
        summary: row.summary,
        payload: row.payload,
        created_at: row.created_at,
    })
}

fn subject_kind(subject_type: ActivitySubjectType) -> &'static str {
    match subject_type {
        ActivitySubjectType::Bookmark => "bookmark",
        ActivitySubjectType::Category => "list",
        ActivitySubjectType::Dashboard => "dashboard",
    }
}

fn default_summary(action: ActivityAction, subject_type: ActivitySubjectType, title: &str) -> String {
    let kind = subject_kind(subject_type);
    let verb = match action {
        ActivityAction::Created => "Created",
        ActivityAction::Updated => "Updated",
        ActivityAction::Deleted => "Deleted",
        ActivityAction::Restored => "Restored",
    };
    let label = if title.trim().is_empty() {
        kind
    } else {
        title.trim()
    };
    format!("{verb} {kind}: {label}")
}

/// Human summary for updates that list concrete field changes.
pub fn update_summary(subject_type: ActivitySubjectType, title: &str, changes: &[String]) -> String {
    let kind = subject_kind(subject_type);
    let label = if title.trim().is_empty() {
        kind.to_string()
    } else {
        format!("“{}”", title.trim())
    };
    if changes.is_empty() {
        format!("Updated {kind} {label}")
    } else {
        format!("Updated {kind} {label}: {}", changes.join("; "))
    }
}

async fn insert_activity(
    pool: &PgPool,
    user_id: Uuid,
    action: ActivityAction,
    subject_type: ActivitySubjectType,
    subject_id: Uuid,
    subject_title: &str,
    summary: &str,
    payload: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO activity_events (
            user_id, action, subject_type, subject_id, subject_title, summary, payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(user_id)
    .bind(action.as_str())
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(subject_title)
    .bind(summary)
    .bind(payload)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Soft-fail wrapper so mutation paths stay resilient.
pub async fn record_activity(
    pool: &PgPool,
    user_id: Uuid,
    action: ActivityAction,
    subject_type: ActivitySubjectType,
    subject_id: Uuid,
    subject_title: &str,
) {
    record_activity_with(
        pool,
        user_id,
        action,
        subject_type,
        subject_id,
        subject_title,
        None,
        json!({}),
    )
    .await;
}

pub async fn record_activity_with(
    pool: &PgPool,
    user_id: Uuid,
    action: ActivityAction,
    subject_type: ActivitySubjectType,
    subject_id: Uuid,
    subject_title: &str,
    summary: Option<&str>,
    payload: serde_json::Value,
) {
    let summary_owned = summary
        .map(str::to_string)
        .unwrap_or_else(|| default_summary(action, subject_type, subject_title));
    if let Err(e) = insert_activity(
        pool,
        user_id,
        action,
        subject_type,
        subject_id,
        subject_title,
        &summary_owned,
        payload,
    )
    .await
    {
        warn!(error = %e, "failed to record activity event");
    }
}

pub async fn list_activity(
    pool: &PgPool,
    user_id: Uuid,
    filter: ListActivityFilter,
) -> Result<Vec<ActivityEvent>> {
    let limit = filter.limit.clamp(1, 200);
    let subject = filter.subject_type.map(|s| s.as_str());
    let action = filter.action.map(|a| a.as_str());

    let rows = sqlx::query_as::<_, ActivityRow>(
        r#"
        SELECT id, user_id, action, subject_type, subject_id, subject_title, summary, payload, created_at
        FROM activity_events
        WHERE user_id = $1
          AND ($2::text IS NULL OR subject_type = $2)
          AND ($3::text IS NULL OR action = $3)
        ORDER BY created_at DESC
        LIMIT $4
        "#,
    )
    .bind(user_id)
    .bind(subject)
    .bind(action)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    rows.into_iter().map(to_event).collect()
}
