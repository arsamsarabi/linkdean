use chrono::{DateTime, Duration, Utc};
use linkdean_core::{
    BanView, ChangeBans, CreateBan, Error, LiftBans, ModerationActionView, Result, UserBan,
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

const MAX_BAN_DAYS: i64 = 3650;
const MAX_REASON_CHARS: usize = 500;

fn db_err(e: sqlx::Error) -> Error {
    Error::msg(e.to_string())
}

fn clean_reason(reason: &str) -> Result<String> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > MAX_REASON_CHARS {
        return Err(Error::BadRequest(
            "reason must be 1 to 500 characters".into(),
        ));
    }
    Ok(reason.to_string())
}

/// A ban ends on a future date, N days from now, or never.
fn resolve_ends_at(
    ends_at: Option<DateTime<Utc>>,
    days: Option<i64>,
) -> Result<Option<DateTime<Utc>>> {
    match (ends_at, days) {
        (Some(_), Some(_)) => Err(Error::BadRequest(
            "give either ends_at or days, not both".into(),
        )),
        (Some(at), None) if at <= Utc::now() => {
            Err(Error::BadRequest("ends_at must be in the future".into()))
        }
        (Some(at), None) => Ok(Some(at)),
        (None, Some(n)) if !(1..=MAX_BAN_DAYS).contains(&n) => {
            Err(Error::BadRequest("days must be between 1 and 3650".into()))
        }
        (None, Some(n)) => Ok(Some(Utc::now() + Duration::days(n))),
        (None, None) => Ok(None),
    }
}

/// The ban in force for a user, if any. Expiry needs no job: it is decided here.
pub async fn active_ban(pool: &PgPool, user_id: Uuid) -> Result<Option<UserBan>> {
    sqlx::query_as::<_, UserBan>(
        r#"
        SELECT * FROM user_bans
        WHERE user_id = $1
          AND lifted_at IS NULL
          AND (ends_at IS NULL OR ends_at > now())
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct BanWindow {
    pub user_id: Uuid,
    pub ends_at: Option<DateTime<Utc>>,
}

pub(super) async fn active_ban_windows(pool: &PgPool) -> Result<Vec<BanWindow>> {
    sqlx::query_as::<_, BanWindow>(
        r#"
        SELECT user_id, ends_at FROM user_bans
        WHERE lifted_at IS NULL AND (ends_at IS NULL OR ends_at > now())
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

/// Admins who can still act, leaving out `user_id`.
pub async fn count_active_admins_except(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let mut conn = pool.acquire().await.map_err(db_err)?;
    active_admins_except(&mut conn, user_id).await
}

async fn active_admins_except(conn: &mut PgConnection, user_id: Uuid) -> Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint FROM users u
        WHERE u.is_admin AND u.id <> $1
          AND NOT EXISTS (
              SELECT 1 FROM user_bans b
              WHERE b.user_id = u.id
                AND b.lifted_at IS NULL
                AND (b.ends_at IS NULL OR b.ends_at > now())
          )
        "#,
    )
    .bind(user_id)
    .fetch_one(conn)
    .await
    .map_err(db_err)
}

pub(super) struct Audit<'a> {
    pub user_id: Uuid,
    pub admin_id: Uuid,
    pub action: &'a str,
    pub subject_type: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    pub subject_title: &'a str,
    pub reason: &'a str,
}

pub(super) async fn record_action(conn: &mut PgConnection, audit: Audit<'_>) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO moderation_actions
            (user_id, admin_id, action, subject_type, subject_id, subject_title, reason)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(audit.user_id)
    .bind(audit.admin_id)
    .bind(audit.action)
    .bind(audit.subject_type)
    .bind(audit.subject_id)
    .bind(audit.subject_title)
    .bind(audit.reason)
    .execute(conn)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn open_ban_id(conn: &mut PgConnection, user_id: Uuid) -> Result<Option<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT id FROM user_bans
        WHERE user_id = $1
          AND lifted_at IS NULL
          AND (ends_at IS NULL OR ends_at > now())
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(conn)
    .await
    .map_err(db_err)
}

/// Bans a user. A lock keeps two admins from banning each other at the same moment.
pub async fn ban_user(
    pool: &PgPool,
    admin_id: Uuid,
    user_id: Uuid,
    req: &CreateBan,
) -> Result<UserBan> {
    if admin_id == user_id {
        return Err(Error::BadRequest("you cannot ban yourself".into()));
    }
    let reason = clean_reason(&req.reason)?;
    let ends_at = resolve_ends_at(req.ends_at, req.days)?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('user_bans'))")
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    let is_admin: bool = sqlx::query_scalar("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    if open_ban_id(&mut tx, user_id).await?.is_some() {
        return Err(Error::Conflict("user is already banned".into()));
    }
    if is_admin && active_admins_except(&mut tx, user_id).await? == 0 {
        return Err(Error::BadRequest("cannot ban the last admin".into()));
    }
    let ban = sqlx::query_as::<_, UserBan>(
        r#"
        INSERT INTO user_bans (user_id, reason, ends_at, created_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&reason)
    .bind(ends_at)
    .bind(admin_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    record_action(
        &mut tx,
        Audit {
            user_id,
            admin_id,
            action: "ban",
            subject_type: None,
            subject_id: None,
            subject_title: "",
            reason: &reason,
        },
    )
    .await?;
    tx.commit().await.map_err(db_err)?;
    Ok(ban)
}

/// Changes reason and duration on every listed ban, or on none of them.
pub async fn change_bans(pool: &PgPool, admin_id: Uuid, req: &ChangeBans) -> Result<()> {
    let reason = req.reason.as_deref().map(clean_reason).transpose()?;
    let change_end = req.permanent || req.ends_at.is_some() || req.days.is_some();
    if req.permanent && (req.ends_at.is_some() || req.days.is_some()) {
        return Err(Error::BadRequest(
            "a permanent ban cannot have an end".into(),
        ));
    }
    if reason.is_none() && !change_end {
        return Err(Error::BadRequest("nothing to change".into()));
    }
    let ends_at = resolve_ends_at(req.ends_at, req.days)?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    for user_id in &req.user_ids {
        let id = open_ban_id(&mut tx, *user_id)
            .await?
            .ok_or_else(|| Error::BadRequest("user has no active ban".into()))?;
        sqlx::query(
            r#"
            UPDATE user_bans
            SET reason = COALESCE($2, reason),
                ends_at = CASE WHEN $3 THEN $4 ELSE ends_at END,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(reason.as_deref())
        .bind(change_end)
        .bind(ends_at)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        record_action(
            &mut tx,
            Audit {
                user_id: *user_id,
                admin_id,
                action: "ban_changed",
                subject_type: None,
                subject_id: None,
                subject_title: "",
                reason: reason.as_deref().unwrap_or(""),
            },
        )
        .await?;
    }
    tx.commit().await.map_err(db_err)
}

/// Lifts every listed ban, or none of them.
pub async fn lift_bans(pool: &PgPool, admin_id: Uuid, req: &LiftBans) -> Result<()> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    for user_id in &req.user_ids {
        let id = open_ban_id(&mut tx, *user_id)
            .await?
            .ok_or_else(|| Error::BadRequest("user has no active ban".into()))?;
        sqlx::query(
            r#"
            UPDATE user_bans
            SET lifted_at = now(), lifted_by = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(admin_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        record_action(
            &mut tx,
            Audit {
                user_id: *user_id,
                admin_id,
                action: "unban",
                subject_type: None,
                subject_id: None,
                subject_title: "",
                reason: "",
            },
        )
        .await?;
    }
    tx.commit().await.map_err(db_err)
}

pub async fn list_bans(pool: &PgPool) -> Result<Vec<BanView>> {
    sqlx::query_as::<_, BanView>(
        r#"
        SELECT b.user_id, u.email, u.display_name, u.is_admin, b.reason, b.ends_at,
               a.email AS created_by_email, b.created_at
        FROM user_bans b
        JOIN users u ON u.id = b.user_id
        LEFT JOIN users a ON a.id = b.created_by
        WHERE b.lifted_at IS NULL AND (b.ends_at IS NULL OR b.ends_at > now())
        ORDER BY b.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

pub async fn list_moderation_actions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ModerationActionView>> {
    sqlx::query_as::<_, ModerationActionView>(
        r#"
        SELECT m.id, m.user_id, m.admin_id, a.email AS admin_email, m.action,
               m.subject_type, m.subject_id, m.subject_title, m.reason, m.created_at
        FROM moderation_actions m
        LEFT JOIN users a ON a.id = m.admin_id
        WHERE m.user_id = $1
        ORDER BY m.created_at DESC
        LIMIT 500
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}
