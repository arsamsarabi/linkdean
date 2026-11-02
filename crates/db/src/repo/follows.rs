use linkdean_core::{
    Error, FollowRelation, FollowUserView, NotificationKind, NotificationSubjectType, Result,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::notifications::{try_notify, NotificationSubject};
use super::users::find_user_by_id;

pub async fn is_following(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> Result<bool> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"
        SELECT 1 FROM user_follows
        WHERE follower_id = $1 AND following_id = $2
        "#,
    )
    .bind(follower_id)
    .bind(following_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.is_some())
}

pub async fn follow_counts(pool: &PgPool, user_id: Uuid) -> Result<(i64, i64)> {
    let followers: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_follows WHERE following_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    let following: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_follows WHERE follower_id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    Ok((followers.0, following.0))
}

pub async fn follow_relation(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<FollowRelation> {
    let (followers, following_count) = follow_counts(pool, target_id).await?;
    let following = if viewer_id == target_id {
        false
    } else {
        is_following(pool, viewer_id, target_id).await?
    };
    Ok(FollowRelation {
        following,
        followers,
        following_count,
    })
}

pub async fn follow_user(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
) -> Result<FollowRelation> {
    if follower_id == following_id {
        return Err(Error::BadRequest("cannot follow yourself".into()));
    }
    let _target = find_user_by_id(pool, following_id)
        .await?
        .ok_or(Error::NotFound)?;
    super::blocks::assert_not_blocked(pool, follower_id, following_id).await?;

    let inserted = sqlx::query(
        r#"
        INSERT INTO user_follows (follower_id, following_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(follower_id)
    .bind(following_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .rows_affected()
        > 0;

    if inserted {
        let actor = find_user_by_id(pool, follower_id)
            .await?
            .ok_or(Error::NotFound)?;
        let actor_label = super::users::public_user_label(&actor, "Someone");
        try_notify(
            pool,
            following_id,
            Some(follower_id),
            NotificationKind::UserFollowed,
            NotificationSubject {
                subject_type: NotificationSubjectType::User,
                subject_id: follower_id,
                subject_title: &actor_label,
            },
            &format!("{actor_label} started following you"),
            json!({ "follower_id": follower_id }),
        )
        .await;
    }

    follow_relation(pool, follower_id, following_id).await
}

pub async fn unfollow_user(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
) -> Result<FollowRelation> {
    if follower_id == following_id {
        return Err(Error::BadRequest("cannot unfollow yourself".into()));
    }
    let _ = find_user_by_id(pool, following_id)
        .await?
        .ok_or(Error::NotFound)?;
    sqlx::query(
        r#"
        DELETE FROM user_follows
        WHERE follower_id = $1 AND following_id = $2
        "#,
    )
    .bind(follower_id)
    .bind(following_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    follow_relation(pool, follower_id, following_id).await
}

async fn map_follow_rows(
    pool: &PgPool,
    rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>)>,
) -> Result<Vec<FollowUserView>> {
    let mut out = Vec::with_capacity(rows.len());
    for (id, created_at) in rows {
        let label = match find_user_by_id(pool, id).await? {
            Some(u) => super::users::public_user_label(&u, "User"),
            None => "User".into(),
        };
        out.push(FollowUserView {
            id,
            display_name: label,
            created_at,
        });
    }
    Ok(out)
}

pub async fn list_followers(pool: &PgPool, user_id: Uuid) -> Result<Vec<FollowUserView>> {
    let rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r#"
        SELECT follower_id, created_at FROM user_follows
        WHERE following_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    map_follow_rows(pool, rows).await
}

pub async fn list_following(pool: &PgPool, user_id: Uuid) -> Result<Vec<FollowUserView>> {
    let rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        r#"
        SELECT following_id, created_at FROM user_follows
        WHERE follower_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    map_follow_rows(pool, rows).await
}
