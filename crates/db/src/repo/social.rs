use linkdean_core::{
    CreateSocialComment, Error, NotificationKind, Result, SocialComment, SocialCommentView,
    SocialSubjectType, SocialSummary, UpsertSocialRating,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::dashboard_shares::require_dashboard_read;
use super::notifications::try_notify;
use super::shares::{bookmark_can_read, require_category_read};
use super::users::{find_user_by_id, share_invitee_label, user_share_label};

async fn actor_label(pool: &PgPool, user_id: Uuid) -> String {
    match find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => user_share_label(&u),
        _ => "Someone".into(),
    }
}

#[derive(Debug, Clone)]
struct SubjectMeta {
    owner_user_id: Uuid,
    title: String,
}

async fn resolve_subject(
    pool: &PgPool,
    user_id: Uuid,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
) -> Result<SubjectMeta> {
    let meta = match subject_type {
        SocialSubjectType::Bookmark => {
            if !bookmark_can_read(pool, user_id, subject_id).await? {
                return Err(Error::NotFound);
            }
            let row: Option<(Uuid, String, bool)> = sqlx::query_as(
                r#"
                SELECT owner_user_id, title, is_vault
                FROM bookmarks
                WHERE id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind(subject_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            let Some((owner, title, is_vault)) = row else {
                return Err(Error::NotFound);
            };
            if is_vault {
                return Err(Error::Forbidden);
            }
            require_shared_context(pool, subject_type, subject_id, owner).await?;
            SubjectMeta {
                owner_user_id: owner,
                title,
            }
        }
        SocialSubjectType::Category => {
            let (cat, _) = require_category_read(pool, user_id, subject_id).await?;
            if cat.is_vault {
                return Err(Error::Forbidden);
            }
            require_shared_context(pool, subject_type, subject_id, cat.owner_user_id).await?;
            SubjectMeta {
                owner_user_id: cat.owner_user_id,
                title: cat.name,
            }
        }
        SocialSubjectType::Dashboard => {
            let (dash, _) = require_dashboard_read(pool, user_id, subject_id).await?;
            if dash.deleted_at.is_some() {
                return Err(Error::NotFound);
            }
            require_shared_context(pool, subject_type, subject_id, dash.owner_user_id).await?;
            SubjectMeta {
                owner_user_id: dash.owner_user_id,
                title: dash.title,
            }
        }
    };
    // Writes and reads require no block with the subject owner.
    super::blocks::assert_not_blocked(pool, user_id, meta.owner_user_id).await?;
    Ok(meta)
}

/// Social only on subjects that participate in sharing (outgoing share or invitee path).
async fn require_shared_context(
    pool: &PgPool,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
    owner_user_id: Uuid,
) -> Result<()> {
    let shared = match subject_type {
        SocialSubjectType::Category => {
            let n: (i64,) = sqlx::query_as(
                "SELECT COUNT(*)::bigint FROM category_shares WHERE category_id = $1",
            )
            .bind(subject_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            n.0 > 0
        }
        SocialSubjectType::Dashboard => {
            let n: (i64,) = sqlx::query_as(
                "SELECT COUNT(*)::bigint FROM dashboard_shares WHERE dashboard_id = $1",
            )
            .bind(subject_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            n.0 > 0
        }
        SocialSubjectType::Bookmark => {
            let n: (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*)::bigint
                FROM category_shares cs
                WHERE cs.category_id IN (
                    SELECT category_id FROM bookmark_categories WHERE bookmark_id = $1
                    UNION
                    SELECT category_id FROM bookmarks WHERE id = $1 AND category_id IS NOT NULL
                )
                "#,
            )
            .bind(subject_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
            n.0 > 0
        }
    };
    if !shared {
        return Err(Error::BadRequest(
            "likes, comments, and ratings require a shared bookmark, list, or dashboard".into(),
        ));
    }
    let _ = owner_user_id;
    Ok(())
}

async fn comment_author_label(pool: &PgPool, user_id: Uuid) -> String {
    match find_user_by_id(pool, user_id).await {
        Ok(Some(u)) => share_invitee_label(u.display_name.as_deref(), &u.email, "User"),
        _ => "User".into(),
    }
}

fn to_comment_view(row: SocialComment, viewer_id: Uuid, author_label: String) -> SocialCommentView {
    SocialCommentView {
        id: row.id,
        user_id: row.user_id,
        author_display_name: author_label,
        body: row.body,
        created_at: row.created_at,
        updated_at: row.updated_at,
        can_delete: row.user_id == viewer_id,
    }
}

pub async fn get_social_summary(
    pool: &PgPool,
    user_id: Uuid,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
) -> Result<SocialSummary> {
    let _meta = resolve_subject(pool, user_id, subject_type, subject_id).await?;
    let st = subject_type.as_str();

    let likes_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint FROM social_likes sl
        WHERE sl.subject_type = $1 AND sl.subject_id = $2
          AND NOT EXISTS (
            SELECT 1 FROM user_blocks ub
            WHERE (ub.blocker_id = $3 AND ub.blocked_id = sl.user_id)
               OR (ub.blocker_id = sl.user_id AND ub.blocked_id = $3)
          )
          AND NOT EXISTS (
            SELECT 1 FROM user_mutes um
            WHERE um.muter_id = $3 AND um.muted_id = sl.user_id
          )
        "#,
    )
    .bind(st)
    .bind(subject_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let liked_by_me: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT user_id FROM social_likes
        WHERE user_id = $1 AND subject_type = $2 AND subject_id = $3
        "#,
    )
    .bind(user_id)
    .bind(st)
    .bind(subject_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let rating_stats: (Option<f64>, i64) = sqlx::query_as(
        r#"
        SELECT AVG(sr.rating)::float8, COUNT(*)::bigint
        FROM social_ratings sr
        WHERE sr.subject_type = $1 AND sr.subject_id = $2
          AND NOT EXISTS (
            SELECT 1 FROM user_blocks ub
            WHERE (ub.blocker_id = $3 AND ub.blocked_id = sr.user_id)
               OR (ub.blocker_id = sr.user_id AND ub.blocked_id = $3)
          )
          AND NOT EXISTS (
            SELECT 1 FROM user_mutes um
            WHERE um.muter_id = $3 AND um.muted_id = sr.user_id
          )
        "#,
    )
    .bind(st)
    .bind(subject_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let my_rating: Option<(i16,)> = sqlx::query_as(
        r#"
        SELECT rating FROM social_ratings
        WHERE user_id = $1 AND subject_type = $2 AND subject_id = $3
        "#,
    )
    .bind(user_id)
    .bind(st)
    .bind(subject_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let comment_rows = sqlx::query_as::<_, SocialComment>(
        r#"
        SELECT sc.* FROM social_comments sc
        WHERE sc.subject_type = $1 AND sc.subject_id = $2
          AND NOT EXISTS (
            SELECT 1 FROM user_blocks ub
            WHERE (ub.blocker_id = $3 AND ub.blocked_id = sc.user_id)
               OR (ub.blocker_id = sc.user_id AND ub.blocked_id = $3)
          )
          AND NOT EXISTS (
            SELECT 1 FROM user_mutes um
            WHERE um.muter_id = $3 AND um.muted_id = sc.user_id
          )
        ORDER BY sc.created_at ASC
        LIMIT 100
        "#,
    )
    .bind(st)
    .bind(subject_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let mut comments = Vec::with_capacity(comment_rows.len());
    for row in comment_rows {
        let label = comment_author_label(pool, row.user_id).await;
        comments.push(to_comment_view(row, user_id, label));
    }

    Ok(SocialSummary {
        subject_type,
        subject_id,
        likes_count: likes_count.0,
        liked_by_me: liked_by_me.is_some(),
        comments_count: comments.len() as i64,
        rating_avg: rating_stats.0,
        rating_count: rating_stats.1,
        my_rating: my_rating.map(|r| r.0),
        comments,
    })
}

pub async fn set_like(
    pool: &PgPool,
    user_id: Uuid,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
    liked: bool,
) -> Result<SocialSummary> {
    let meta = resolve_subject(pool, user_id, subject_type, subject_id).await?;
    let st = subject_type.as_str();
    if liked {
        sqlx::query(
            r#"
            INSERT INTO social_likes (user_id, subject_type, subject_id)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(st)
        .bind(subject_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

        let actor = actor_label(pool, user_id).await;
        try_notify(
            pool,
            meta.owner_user_id,
            Some(user_id),
            NotificationKind::SocialLike,
            subject_type.to_notification_subject(),
            subject_id,
            &meta.title,
            &format!("{actor} liked “{}”", meta.title),
            json!({}),
        )
        .await;
    } else {
        sqlx::query(
            r#"
            DELETE FROM social_likes
            WHERE user_id = $1 AND subject_type = $2 AND subject_id = $3
            "#,
        )
        .bind(user_id)
        .bind(st)
        .bind(subject_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }
    get_social_summary(pool, user_id, subject_type, subject_id).await
}

pub async fn add_comment(
    pool: &PgPool,
    user_id: Uuid,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
    req: CreateSocialComment,
) -> Result<SocialSummary> {
    let body = req.body.trim().to_string();
    if body.is_empty() || body.chars().count() > 4000 {
        return Err(Error::BadRequest(
            "comment must be 1–4000 characters".into(),
        ));
    }
    let meta = resolve_subject(pool, user_id, subject_type, subject_id).await?;
    sqlx::query(
        r#"
        INSERT INTO social_comments (user_id, subject_type, subject_id, body)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(&body)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let actor = actor_label(pool, user_id).await;
    let preview: String = body.chars().take(80).collect();
    try_notify(
        pool,
        meta.owner_user_id,
        Some(user_id),
        NotificationKind::SocialComment,
        subject_type.to_notification_subject(),
        subject_id,
        &meta.title,
        &format!("{actor} commented on “{}”: {preview}", meta.title),
        json!({}),
    )
    .await;

    get_social_summary(pool, user_id, subject_type, subject_id).await
}

pub async fn delete_comment(pool: &PgPool, user_id: Uuid, comment_id: Uuid) -> Result<()> {
    let row = sqlx::query_as::<_, SocialComment>("SELECT * FROM social_comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let Some(row) = row else {
        return Err(Error::NotFound);
    };
    if row.user_id != user_id {
        return Err(Error::Forbidden);
    }
    let subject_type = SocialSubjectType::parse(&row.subject_type).map_err(Error::BadRequest)?;
    // Ensure still readable / shared before delete (soft check).
    let _ = resolve_subject(pool, user_id, subject_type, row.subject_id).await?;
    sqlx::query("DELETE FROM social_comments WHERE id = $1")
        .bind(comment_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn upsert_rating(
    pool: &PgPool,
    user_id: Uuid,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
    req: UpsertSocialRating,
) -> Result<SocialSummary> {
    if !(1..=5).contains(&req.rating) {
        return Err(Error::BadRequest("rating must be 1–5".into()));
    }
    let meta = resolve_subject(pool, user_id, subject_type, subject_id).await?;
    sqlx::query(
        r#"
        INSERT INTO social_ratings (user_id, subject_type, subject_id, rating)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, subject_type, subject_id) DO UPDATE
        SET rating = EXCLUDED.rating, updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(subject_type.as_str())
    .bind(subject_id)
    .bind(req.rating)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let actor = actor_label(pool, user_id).await;
    try_notify(
        pool,
        meta.owner_user_id,
        Some(user_id),
        NotificationKind::SocialRating,
        subject_type.to_notification_subject(),
        subject_id,
        &meta.title,
        &format!("{actor} rated “{}” {}/5", meta.title, req.rating),
        json!({ "rating": req.rating }),
    )
    .await;

    get_social_summary(pool, user_id, subject_type, subject_id).await
}

pub async fn clear_rating(
    pool: &PgPool,
    user_id: Uuid,
    subject_type: SocialSubjectType,
    subject_id: Uuid,
) -> Result<SocialSummary> {
    let _meta = resolve_subject(pool, user_id, subject_type, subject_id).await?;
    sqlx::query(
        r#"
        DELETE FROM social_ratings
        WHERE user_id = $1 AND subject_type = $2 AND subject_id = $3
        "#,
    )
    .bind(user_id)
    .bind(subject_type.as_str())
    .bind(subject_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    get_social_summary(pool, user_id, subject_type, subject_id).await
}
