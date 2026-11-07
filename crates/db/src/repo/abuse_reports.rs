//! Abuse reports: filed by signed-in users, worked by admins.
//!
//! A report snapshots the reported user and a label when it is filed, so the admin queue still
//! reads right after the target is hidden, deleted or its public link revoked.

use linkdean_core::{
    AbuseAction, AbuseReport, AbuseTargetType, CreateAbuseReport, CreateBan, Error,
    ResolveAbuseReport, Result, SocialSubjectType,
};
use sqlx::AssertSqlSafe;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::moderation::{ban_user, record_action, Audit};
use super::profiles::require_viewable_profile;
use super::social::resolve_subject;

/// The queue shows at most this many reports, newest first.
const LIST_LIMIT: i64 = 200;
const LABEL_CHARS: usize = 80;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

const REPORT_SELECT: &str = r#"
    SELECT r.id, r.reporter_user_id, u.email AS reporter_email, r.target_type, r.target_id,
           r.target_user_id, r.target_label, r.reason, r.details, r.status,
           r.resolution_action, r.resolution_note, r.resolved_by, r.created_at, r.resolved_at
    FROM abuse_reports r
    LEFT JOIN users u ON u.id = r.reporter_user_id
"#;

/// A target the reporter may see, as it will be stored.
struct Target {
    id: String,
    owner_id: Uuid,
    label: String,
}

fn parse_id(raw: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).map_err(|_| Error::BadRequest("target_id is not a valid id".into()))
}

fn excerpt(text: &str) -> String {
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= LABEL_CHARS {
        return text;
    }
    let cut: String = text.chars().take(LABEL_CHARS).collect();
    format!("{cut}…")
}

/// The target when it exists and `reporter_id` could see it. Denials are `NotFound`, so a
/// report cannot probe for hidden content.
async fn resolve_target(
    pool: &PgPool,
    reporter_id: Uuid,
    kind: AbuseTargetType,
    raw_id: &str,
) -> Result<Target> {
    let target = match kind {
        AbuseTargetType::PublicList => {
            // A slug or a category id, but only while the list is published.
            let row: Option<(Uuid, Uuid, String)> = sqlx::query_as(
                r#"
                SELECT id, owner_user_id, name FROM categories
                WHERE (public_slug = $1 OR id::text = $1)
                  AND public_slug IS NOT NULL
                  AND is_vault = FALSE
                  AND deleted_at IS NULL
                "#,
            )
            .bind(raw_id)
            .fetch_optional(pool)
            .await
            .map_err(db_err)?;
            let (id, owner_id, name) = row.ok_or(Error::NotFound)?;
            Target {
                id: id.to_string(),
                owner_id,
                label: excerpt(&name),
            }
        }
        AbuseTargetType::Profile => {
            let user = require_viewable_profile(pool, reporter_id, parse_id(raw_id)?).await?;
            Target {
                id: user.id.to_string(),
                owner_id: user.id,
                label: super::users::public_user_label(&user, "User"),
            }
        }
        AbuseTargetType::Comment => {
            let id = parse_id(raw_id)?;
            let row: Option<(Uuid, String, Uuid, String)> = sqlx::query_as(
                "SELECT user_id, subject_type, subject_id, body FROM social_comments WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(db_err)?;
            let (author_id, subject_type, subject_id, body) = row.ok_or(Error::NotFound)?;
            let subject_type =
                SocialSubjectType::parse(&subject_type).map_err(Error::BadRequest)?;
            // Only people who can read the thread may report its comments.
            match resolve_subject(pool, reporter_id, subject_type, subject_id).await {
                Ok(_) => {}
                Err(Error::Forbidden | Error::BadRequest(_)) => return Err(Error::NotFound),
                Err(e) => return Err(e),
            }
            Target {
                id: id.to_string(),
                owner_id: author_id,
                label: excerpt(&body),
            }
        }
    };
    if target.owner_id == reporter_id {
        return Err(Error::BadRequest(
            "you cannot report your own content".into(),
        ));
    }
    Ok(target)
}

async fn fetch_report(pool: &PgPool, id: Uuid) -> Result<Option<AbuseReport>> {
    sqlx::query_as::<_, AbuseReport>(AssertSqlSafe(format!("{REPORT_SELECT} WHERE r.id = $1")))
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(db_err)
}

/// Files a report. Reporting the same target again while your report is open returns that one.
pub async fn create_report(
    pool: &PgPool,
    reporter_id: Uuid,
    req: &CreateAbuseReport,
) -> Result<AbuseReport> {
    let (raw_id, reason, details) = req.validate()?;
    let target = resolve_target(pool, reporter_id, req.target_type, &raw_id).await?;

    let open: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM abuse_reports
        WHERE reporter_user_id = $1 AND target_type = $2 AND target_id = $3 AND status = 'open'
        LIMIT 1
        "#,
    )
    .bind(reporter_id)
    .bind(req.target_type.as_str())
    .bind(&target.id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;
    let id = match open {
        Some(id) => id,
        None => sqlx::query_scalar(
            r#"
            INSERT INTO abuse_reports
                (reporter_user_id, target_type, target_id, target_user_id, target_label,
                 reason, details)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(reporter_id)
        .bind(req.target_type.as_str())
        .bind(&target.id)
        .bind(target.owner_id)
        .bind(&target.label)
        .bind(&reason)
        .bind(details)
        .fetch_one(pool)
        .await
        .map_err(db_err)?,
    };
    fetch_report(pool, id)
        .await?
        .ok_or_else(|| Error::msg("abuse report vanished after insert"))
}

/// Reports with `status` (`open` or `resolved`), newest first.
pub async fn list_reports(pool: &PgPool, status: &str) -> Result<Vec<AbuseReport>> {
    if !matches!(status, "open" | "resolved") {
        return Err(Error::BadRequest("status must be open or resolved".into()));
    }
    sqlx::query_as::<_, AbuseReport>(AssertSqlSafe(format!(
        "{REPORT_SELECT} WHERE r.status = $1 ORDER BY r.created_at DESC LIMIT $2"
    )))
    .bind(status)
    .bind(LIST_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

/// Takes the target down: revokes a public link, hides a profile or deletes a comment. Doing
/// it twice is not an error.
async fn hide_target(
    conn: &mut PgConnection,
    admin_id: Uuid,
    report: &AbuseReport,
    reason: &str,
) -> Result<()> {
    let id = parse_id(&report.target_id)?;
    let (changed, action, subject_type) = match report.target_type.as_str() {
        "public_list" => (
            sqlx::query(
                r#"
                UPDATE categories SET public_slug = NULL, updated_at = now()
                WHERE id = $1 AND public_slug IS NOT NULL
                "#,
            )
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db_err)?
            .rows_affected(),
            "public_link_disabled",
            "category",
        ),
        "profile" => (
            sqlx::query(
                r#"
                UPDATE users SET profile_visibility = 'hidden', updated_at = now()
                WHERE id = $1 AND profile_visibility <> 'hidden'
                "#,
            )
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db_err)?
            .rows_affected(),
            "profile_hidden",
            "user",
        ),
        "comment" => (
            sqlx::query("DELETE FROM social_comments WHERE id = $1")
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db_err)?
                .rows_affected(),
            "comment_deleted",
            "comment",
        ),
        other => return Err(Error::msg(format!("unknown abuse target type {other}"))),
    };
    // The audit trail is per affected user; with the user gone there is nobody to attach it to.
    if let (true, Some(user_id)) = (changed > 0, report.target_user_id) {
        record_action(
            conn,
            Audit {
                user_id,
                admin_id,
                action,
                subject_type: Some(subject_type),
                subject_id: Some(id),
                subject_title: &report.target_label,
                reason,
            },
        )
        .await?;
    }
    Ok(())
}

/// Resolves an open report. `ban` uses the note as the ban reason, else the report's reason; a
/// user who is already banned stays banned and the report still resolves. `warn` only records
/// the note; the caller tells the user.
pub async fn resolve_report(
    pool: &PgPool,
    admin_id: Uuid,
    id: Uuid,
    req: &ResolveAbuseReport,
) -> Result<AbuseReport> {
    let note = req.validate()?;
    let report = fetch_report(pool, id).await?.ok_or(Error::NotFound)?;
    if report.status != "open" {
        return Err(Error::Conflict("report is already resolved".into()));
    }
    let reason = note
        .clone()
        .unwrap_or_else(|| format!("Abuse report: {}", report.reason));

    if matches!(req.action, AbuseAction::Warn | AbuseAction::Ban) && report.target_user_id.is_none()
    {
        return Err(Error::BadRequest(
            "the reported user no longer exists".into(),
        ));
    }
    if let (AbuseAction::Ban, Some(user_id)) = (req.action, report.target_user_id) {
        let ban = CreateBan {
            reason: reason.clone(),
            ..CreateBan::default()
        };
        match ban_user(pool, admin_id, user_id, &ban).await {
            Ok(_) | Err(Error::Conflict(_)) => {}
            Err(e) => return Err(e),
        }
    }

    let mut tx = pool.begin().await.map_err(db_err)?;
    if req.action == AbuseAction::Hide {
        hide_target(&mut tx, admin_id, &report, &reason).await?;
    }
    let claimed = sqlx::query(
        r#"
        UPDATE abuse_reports
        SET status = 'resolved', resolution_action = $2, resolution_note = $3,
            resolved_by = $4, resolved_at = now()
        WHERE id = $1 AND status = 'open'
        "#,
    )
    .bind(id)
    .bind(req.action.as_str())
    .bind(note)
    .bind(admin_id)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?
    .rows_affected();
    if claimed == 0 {
        return Err(Error::Conflict("report is already resolved".into()));
    }
    tx.commit().await.map_err(db_err)?;
    fetch_report(pool, id).await?.ok_or(Error::NotFound)
}
