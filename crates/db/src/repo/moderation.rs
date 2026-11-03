use chrono::{DateTime, Duration, Utc};
use linkdean_core::{
    BanView, ChangeBans, CreateBan, Error, ExposedBookmark, ExposedCategory, ExposedComment,
    ExposedContent, ExposedShare, LiftBans, ModerateAction, ModerateOutcome, ModerateRequest,
    ModerationActionView, Result, UserBan,
};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use std::collections::BTreeMap;
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

/// Admins who can still act, leaving out `user_id`. Call under the `user_bans` advisory lock.
pub(crate) async fn active_admins_except(conn: &mut PgConnection, user_id: Uuid) -> Result<i64> {
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

const MAX_EXPOSED_ROWS: i64 = 500;

/// What a user shows to other people. Only for a user with an active ban.
pub async fn exposed_content(pool: &PgPool, user_id: Uuid) -> Result<ExposedContent> {
    if active_ban(pool, user_id).await?.is_none() {
        return Err(Error::Conflict("user has no active ban".into()));
    }
    let categories = sqlx::query_as::<_, ExposedCategory>(
        r#"
        SELECT id, name, visibility, public_slug FROM categories
        WHERE owner_user_id = $1 AND deleted_at IS NULL AND NOT is_vault
          AND (visibility <> 'private' OR public_slug IS NOT NULL)
        ORDER BY name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let public_links = categories
        .iter()
        .filter(|c| c.public_slug.is_some())
        .cloned()
        .collect();
    let categories = categories
        .into_iter()
        .filter(|c| c.visibility != "private")
        .collect();

    let bookmarks = sqlx::query_as::<_, ExposedBookmark>(
        r#"
        WITH RECURSIVE exposed AS (
            SELECT c.id FROM categories c
            WHERE c.owner_user_id = $1 AND c.deleted_at IS NULL AND NOT c.is_vault
              AND (
                  c.visibility <> 'private'
                  OR c.public_slug IS NOT NULL
                  OR EXISTS (SELECT 1 FROM category_shares s WHERE s.category_id = c.id)
              )
            UNION
            SELECT c.id FROM categories c
            JOIN exposed e ON c.parent_id = e.id
            WHERE c.owner_user_id = $1 AND c.deleted_at IS NULL AND NOT c.is_vault
        )
        SELECT b.id, b.title, b.url FROM bookmarks b
        WHERE b.owner_user_id = $1
          AND b.deleted_at IS NULL
          AND NOT b.is_vault
          AND NOT b.is_private
          AND EXISTS (
              SELECT 1 FROM bookmark_categories bc
              JOIN exposed e ON e.id = bc.category_id
              WHERE bc.bookmark_id = b.id
          )
        ORDER BY b.updated_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(MAX_EXPOSED_ROWS)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let profile: String = sqlx::query_scalar("SELECT profile_visibility FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

    let shares = sqlx::query_as::<_, ExposedShare>(
        r#"
        SELECT 'category_share' AS kind, s.id, c.name AS title, u.email AS invitee, s.access
        FROM category_shares s
        JOIN categories c ON c.id = s.category_id
        JOIN users u ON u.id = s.invitee_user_id
        WHERE c.owner_user_id = $1 AND NOT c.is_vault
        UNION ALL
        SELECT 'dashboard_share', s.id, d.title, u.email, s.access
        FROM dashboard_shares s
        JOIN dashboards d ON d.id = s.dashboard_id
        JOIN users u ON u.id = s.invitee_user_id
        WHERE d.owner_user_id = $1
        UNION ALL
        SELECT 'pending_share', p.id,
               COALESCE(
                   (SELECT c.name FROM categories c WHERE c.id = p.subject_id),
                   (SELECT d.title FROM dashboards d WHERE d.id = p.subject_id),
                   ''
               ),
               p.invitee_email, p.access
        FROM pending_share_invites p
        WHERE p.invited_by_user_id = $1 AND p.claimed_at IS NULL AND p.revoked_at IS NULL
        ORDER BY title
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(MAX_EXPOSED_ROWS)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let comments = sqlx::query_as::<_, ExposedComment>(
        r#"
        SELECT id, body, subject_type, created_at FROM social_comments
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(MAX_EXPOSED_ROWS)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    Ok(ExposedContent {
        public_links,
        categories,
        bookmarks,
        profile_visibility: (profile != "hidden").then_some(profile),
        shares,
        comments,
    })
}

#[derive(sqlx::FromRow)]
struct CategoryState {
    owner_user_id: Uuid,
    name: String,
    is_vault: bool,
    visibility: String,
    public_slug: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct BookmarkState {
    owner_user_id: Uuid,
    title: String,
    url: String,
    is_vault: bool,
    is_private: bool,
    deleted_at: Option<DateTime<Utc>>,
}

/// A row that cannot be acted on stops the whole request.
async fn owned_category(conn: &mut PgConnection, user_id: Uuid, id: Uuid) -> Result<CategoryState> {
    let row = sqlx::query_as::<_, CategoryState>(
        r#"
        SELECT owner_user_id, name, is_vault, visibility, public_slug, deleted_at
        FROM categories WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db_err)?
    .filter(|c| c.owner_user_id == user_id)
    .ok_or(Error::NotFound)?;
    if row.is_vault {
        return Err(Error::BadRequest("vault items cannot be moderated".into()));
    }
    Ok(row)
}

async fn owned_bookmark(conn: &mut PgConnection, user_id: Uuid, id: Uuid) -> Result<BookmarkState> {
    let row = sqlx::query_as::<_, BookmarkState>(
        r#"
        SELECT owner_user_id, title, url, is_vault, is_private, deleted_at
        FROM bookmarks WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db_err)?
    .filter(|b| b.owner_user_id == user_id)
    .ok_or(Error::NotFound)?;
    if row.is_vault {
        return Err(Error::BadRequest("vault items cannot be moderated".into()));
    }
    Ok(row)
}

async fn set_category_column(conn: &mut PgConnection, id: Uuid, assignment: &str) -> Result<()> {
    sqlx::query(&format!(
        "UPDATE categories SET {assignment}, updated_at = now() WHERE id = $1"
    ))
    .bind(id)
    .execute(conn)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn trash_category(conn: &mut PgConnection, user_id: Uuid, id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        WITH RECURSIVE tree AS (
            SELECT id FROM categories WHERE id = $1
            UNION
            SELECT c.id FROM categories c
            JOIN tree t ON c.parent_id = t.id
            WHERE c.owner_user_id = $2 AND c.deleted_at IS NULL
        )
        UPDATE categories
        SET deleted_at = now(), updated_at = now()
        WHERE id IN (SELECT id FROM tree)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(db_err)?;
    Ok(())
}

/// A revoked share, so its invitee can be told after the commit.
struct Revoked {
    is_dashboard: bool,
    invitee_user_id: Uuid,
    subject_id: Uuid,
    title: String,
}

async fn revoke_share(
    conn: &mut PgConnection,
    user_id: Uuid,
    kind: Option<&str>,
    id: Uuid,
    revoked: &mut Vec<Revoked>,
) -> Result<Option<(&'static str, String)>> {
    let (kind, table, join, owner) = match kind {
        Some("category_share") => (
            "category_share",
            "category_shares",
            "categories p ON p.id = s.category_id",
            "p.name",
        ),
        Some("dashboard_share") => (
            "dashboard_share",
            "dashboard_shares",
            "dashboards p ON p.id = s.dashboard_id",
            "p.title",
        ),
        Some("pending_share") => return revoke_pending_share(conn, user_id, id).await,
        _ => return Err(Error::BadRequest("unknown share type".into())),
    };
    let parent = if kind == "category_share" {
        "s.category_id"
    } else {
        "s.dashboard_id"
    };
    let row: Option<(Uuid, Uuid, Uuid, String)> = sqlx::query_as(&format!(
        r#"
        SELECT p.owner_user_id, s.invitee_user_id, {parent}, {owner}
        FROM {table} s JOIN {join}
        WHERE s.id = $1
        "#
    ))
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?;
    let Some((owner_id, invitee_user_id, subject_id, title)) = row else {
        return Ok(None);
    };
    if owner_id != user_id {
        return Err(Error::NotFound);
    }
    sqlx::query(&format!("DELETE FROM {table} WHERE id = $1"))
        .bind(id)
        .execute(conn)
        .await
        .map_err(db_err)?;
    revoked.push(Revoked {
        is_dashboard: kind == "dashboard_share",
        invitee_user_id,
        subject_id,
        title: title.clone(),
    });
    Ok(Some((kind, title)))
}

async fn revoke_pending_share(
    conn: &mut PgConnection,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<(&'static str, String)>> {
    let row: Option<(Uuid, String, bool)> = sqlx::query_as(
        r#"
        SELECT invited_by_user_id, invitee_email, claimed_at IS NULL AND revoked_at IS NULL
        FROM pending_share_invites WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?;
    let (inviter, email, open) = row.ok_or(Error::NotFound)?;
    if inviter != user_id {
        return Err(Error::NotFound);
    }
    if !open {
        return Ok(None);
    }
    sqlx::query(
        "UPDATE pending_share_invites SET revoked_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(conn)
    .await
    .map_err(db_err)?;
    Ok(Some(("pending_share", email)))
}

async fn delete_comment_as_admin(
    conn: &mut PgConnection,
    user_id: Uuid,
    id: Uuid,
) -> Result<Option<(&'static str, String)>> {
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT user_id, body FROM social_comments WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db_err)?;
    let Some((author, body)) = row else {
        return Ok(None);
    };
    if author != user_id {
        return Err(Error::NotFound);
    }
    sqlx::query("DELETE FROM social_comments WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await
        .map_err(db_err)?;
    Ok(Some(("comment", body)))
}

/// Applies one action. `None` means it was already done, which is not an error.
async fn apply_action(
    conn: &mut PgConnection,
    user_id: Uuid,
    item: &ModerateAction,
    outcome: &mut ModerateOutcome,
    revoked: &mut Vec<Revoked>,
) -> Result<Option<(&'static str, String)>> {
    let id = item.subject_id;
    match item.action.as_str() {
        "public_link_disabled" => {
            let c = owned_category(conn, user_id, id).await?;
            if c.public_slug.is_none() {
                return Ok(None);
            }
            set_category_column(conn, id, "public_slug = NULL").await?;
            Ok(Some(("category", c.name)))
        }
        "category_private" => {
            let c = owned_category(conn, user_id, id).await?;
            if c.visibility == "private" {
                return Ok(None);
            }
            set_category_column(conn, id, "visibility = 'private'").await?;
            Ok(Some(("category", c.name)))
        }
        "profile_hidden" => {
            if id != user_id {
                return Err(Error::NotFound);
            }
            let res = sqlx::query(
                r#"
                UPDATE users SET profile_visibility = 'hidden', updated_at = now()
                WHERE id = $1 AND profile_visibility <> 'hidden'
                "#,
            )
            .bind(id)
            .execute(conn)
            .await
            .map_err(db_err)?;
            Ok((res.rows_affected() > 0).then(|| ("user", String::new())))
        }
        "share_revoked" => {
            revoke_share(conn, user_id, item.subject_type.as_deref(), id, revoked).await
        }
        "comment_deleted" => delete_comment_as_admin(conn, user_id, id).await,
        "bookmark_private" => {
            let b = owned_bookmark(conn, user_id, id).await?;
            if b.is_private {
                return Ok(None);
            }
            sqlx::query("UPDATE bookmarks SET is_private = TRUE, updated_at = now() WHERE id = $1")
                .bind(id)
                .execute(conn)
                .await
                .map_err(db_err)?;
            Ok(Some(("bookmark", display_title(b.title, b.url))))
        }
        "removed_to_trash" => match item.subject_type.as_deref() {
            Some("bookmark") => {
                let b = owned_bookmark(conn, user_id, id).await?;
                if b.deleted_at.is_some() {
                    return Ok(None);
                }
                sqlx::query(
                    "UPDATE bookmarks SET deleted_at = now(), updated_at = now() WHERE id = $1",
                )
                .bind(id)
                .execute(conn)
                .await
                .map_err(db_err)?;
                outcome.trashed_bookmark_ids.push(id);
                Ok(Some(("bookmark", display_title(b.title, b.url))))
            }
            Some("category") => {
                let c = owned_category(conn, user_id, id).await?;
                if c.deleted_at.is_some() {
                    return Ok(None);
                }
                trash_category(conn, user_id, id).await?;
                Ok(Some(("category", c.name)))
            }
            _ => Err(Error::BadRequest("unknown trash subject".into())),
        },
        _ => Err(Error::BadRequest("unknown moderation action".into())),
    }
}

fn display_title(title: String, url: String) -> String {
    if title.is_empty() {
        url
    } else {
        title
    }
}

/// Applies every action or none, records each one, and tells the user once.
pub async fn moderate_content(
    pool: &PgPool,
    admin_id: Uuid,
    user_id: Uuid,
    req: &ModerateRequest,
) -> Result<ModerateOutcome> {
    let reason = clean_reason(&req.reason)?;
    if req.actions.is_empty() {
        return Err(Error::BadRequest("no actions given".into()));
    }
    let mut tx = pool.begin().await.map_err(db_err)?;
    if open_ban_id(&mut tx, user_id).await?.is_none() {
        return Err(Error::Conflict("user has no active ban".into()));
    }
    let mut outcome = ModerateOutcome::default();
    let mut revoked = Vec::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for item in &req.actions {
        let applied = apply_action(&mut tx, user_id, item, &mut outcome, &mut revoked).await?;
        let Some((subject_type, title)) = applied else {
            outcome.skipped += 1;
            continue;
        };
        outcome.applied += 1;
        *counts.entry(item.action.as_str()).or_default() += 1;
        record_action(
            &mut tx,
            Audit {
                user_id,
                admin_id,
                action: &item.action,
                subject_type: Some(subject_type),
                subject_id: Some(item.subject_id),
                subject_title: &title,
                reason: &reason,
            },
        )
        .await?;
    }
    if outcome.applied > 0 {
        sqlx::query(
            r#"
            INSERT INTO notifications
                (recipient_user_id, actor_user_id, kind, subject_type, subject_id, body, payload)
            VALUES ($1, $2, 'moderation_changes', 'user', $1, $3, $4)
            "#,
        )
        .bind(user_id)
        .bind(admin_id)
        .bind(format!(
            "An admin changed {} of your items: {reason}",
            outcome.applied
        ))
        .bind(json!({ "counts": counts }))
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;

    for r in revoked {
        if r.is_dashboard {
            super::notifications::notify_dashboard_share_revoked(
                pool,
                admin_id,
                r.invitee_user_id,
                r.subject_id,
                &r.title,
            )
            .await;
        } else {
            super::notifications::notify_category_share_revoked(
                pool,
                admin_id,
                r.invitee_user_id,
                r.subject_id,
                &r.title,
            )
            .await;
        }
    }
    Ok(outcome)
}
