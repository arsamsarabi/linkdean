use linkdean_core::{BlockRelation, Error, FollowUserView, MuteRelation, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::users::find_user_by_id;

pub async fn is_blocked(pool: &PgPool, blocker_id: Uuid, blocked_id: Uuid) -> Result<bool> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"
        SELECT 1 FROM user_blocks
        WHERE blocker_id = $1 AND blocked_id = $2
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.is_some())
}

pub async fn is_blocked_either_way(pool: &PgPool, a: Uuid, b: Uuid) -> Result<bool> {
    if a == b {
        return Ok(false);
    }
    let row: Option<(i32,)> = sqlx::query_as(
        r#"
        SELECT 1 FROM user_blocks
        WHERE (blocker_id = $1 AND blocked_id = $2)
           OR (blocker_id = $2 AND blocked_id = $1)
        "#,
    )
    .bind(a)
    .bind(b)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.is_some())
}

pub async fn assert_not_blocked(pool: &PgPool, a: Uuid, b: Uuid) -> Result<()> {
    if is_blocked_either_way(pool, a, b).await? {
        return Err(Error::Forbidden);
    }
    Ok(())
}

pub async fn is_muted(pool: &PgPool, muter_id: Uuid, muted_id: Uuid) -> Result<bool> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"
        SELECT 1 FROM user_mutes
        WHERE muter_id = $1 AND muted_id = $2
        "#,
    )
    .bind(muter_id)
    .bind(muted_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(row.is_some())
}

/// Viewer should not see actor's activity (block either way, or mute by viewer).
pub async fn viewer_hides_user(pool: &PgPool, viewer_id: Uuid, other_id: Uuid) -> Result<bool> {
    if viewer_id == other_id {
        return Ok(false);
    }
    if is_blocked_either_way(pool, viewer_id, other_id).await? {
        return Ok(true);
    }
    is_muted(pool, viewer_id, other_id).await
}

pub async fn block_relation(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<BlockRelation> {
    let blocked = if viewer_id == target_id {
        false
    } else {
        is_blocked(pool, viewer_id, target_id).await?
    };
    Ok(BlockRelation { blocked })
}

pub async fn mute_relation(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<MuteRelation> {
    let muted = if viewer_id == target_id {
        false
    } else {
        is_muted(pool, viewer_id, target_id).await?
    };
    Ok(MuteRelation { muted })
}

pub async fn block_user(
    pool: &PgPool,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<BlockRelation> {
    if blocker_id == blocked_id {
        return Err(Error::BadRequest("cannot block yourself".into()));
    }
    let target = find_user_by_id(pool, blocked_id)
        .await?
        .ok_or(Error::NotFound)?;
    let blocker = find_user_by_id(pool, blocker_id)
        .await?
        .ok_or(Error::NotFound)?;

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO user_blocks (blocker_id, blocked_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Mutual unfollow.
    sqlx::query(
        r#"
        DELETE FROM user_follows
        WHERE (follower_id = $1 AND following_id = $2)
           OR (follower_id = $2 AND following_id = $1)
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Clear mutes either way (block supersedes).
    sqlx::query(
        r#"
        DELETE FROM user_mutes
        WHERE (muter_id = $1 AND muted_id = $2)
           OR (muter_id = $2 AND muted_id = $1)
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Silent revoke category shares both directions.
    sqlx::query(
        r#"
        DELETE FROM category_shares cs
        USING categories c
        WHERE cs.category_id = c.id
          AND (
            (c.owner_user_id = $1 AND cs.invitee_user_id = $2)
            OR (c.owner_user_id = $2 AND cs.invitee_user_id = $1)
          )
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Silent revoke dashboard shares both directions.
    sqlx::query(
        r#"
        DELETE FROM dashboard_shares ds
        USING dashboards d
        WHERE ds.dashboard_id = d.id
          AND (
            (d.owner_user_id = $1 AND ds.invitee_user_id = $2)
            OR (d.owner_user_id = $2 AND ds.invitee_user_id = $1)
          )
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    // Silent revoke open invites both directions (by email or account).
    sqlx::query(
        r#"
        UPDATE pending_share_invites
        SET revoked_at = now(), updated_at = now()
        WHERE claimed_at IS NULL
          AND revoked_at IS NULL
          AND (
            (invited_by_user_id = $1 AND (invitee_email = $2 OR invitee_user_id = $5))
            OR (invited_by_user_id = $3 AND (invitee_email = $4 OR invitee_user_id = $6))
          )
        "#,
    )
    .bind(blocker_id)
    .bind(target.email.to_ascii_lowercase())
    .bind(blocked_id)
    .bind(blocker.email.to_ascii_lowercase())
    .bind(blocked_id)
    .bind(blocker_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    block_relation(pool, blocker_id, blocked_id).await
}

pub async fn unblock_user(
    pool: &PgPool,
    blocker_id: Uuid,
    blocked_id: Uuid,
) -> Result<BlockRelation> {
    if blocker_id == blocked_id {
        return Err(Error::BadRequest("cannot unblock yourself".into()));
    }
    let _ = find_user_by_id(pool, blocked_id)
        .await?
        .ok_or(Error::NotFound)?;
    sqlx::query(
        r#"
        DELETE FROM user_blocks
        WHERE blocker_id = $1 AND blocked_id = $2
        "#,
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    block_relation(pool, blocker_id, blocked_id).await
}

pub async fn mute_user(pool: &PgPool, muter_id: Uuid, muted_id: Uuid) -> Result<MuteRelation> {
    if muter_id == muted_id {
        return Err(Error::BadRequest("cannot mute yourself".into()));
    }
    let _ = find_user_by_id(pool, muted_id)
        .await?
        .ok_or(Error::NotFound)?;
    if is_blocked_either_way(pool, muter_id, muted_id).await? {
        return Err(Error::BadRequest("cannot mute a blocked user".into()));
    }
    sqlx::query(
        r#"
        INSERT INTO user_mutes (muter_id, muted_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(muter_id)
    .bind(muted_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    mute_relation(pool, muter_id, muted_id).await
}

pub async fn unmute_user(pool: &PgPool, muter_id: Uuid, muted_id: Uuid) -> Result<MuteRelation> {
    if muter_id == muted_id {
        return Err(Error::BadRequest("cannot unmute yourself".into()));
    }
    let _ = find_user_by_id(pool, muted_id)
        .await?
        .ok_or(Error::NotFound)?;
    sqlx::query(
        r#"
        DELETE FROM user_mutes
        WHERE muter_id = $1 AND muted_id = $2
        "#,
    )
    .bind(muter_id)
    .bind(muted_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    mute_relation(pool, muter_id, muted_id).await
}

type TargetRow = (Uuid, Option<String>, String, chrono::DateTime<chrono::Utc>);

fn to_user_views(rows: Vec<TargetRow>) -> Vec<FollowUserView> {
    rows.into_iter()
        .map(|(id, display_name, _email, created_at)| FollowUserView {
            id,
            // Blocked and muted people may never have shared an email with the viewer.
            display_name: display_name
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "User".into()),
            created_at,
        })
        .collect()
}

pub async fn list_blocked(pool: &PgPool, user_id: Uuid) -> Result<Vec<FollowUserView>> {
    let rows: Vec<TargetRow> = sqlx::query_as(
        r#"
        SELECT u.id, u.display_name, u.email, ub.created_at
        FROM user_blocks ub
        JOIN users u ON u.id = ub.blocked_id
        WHERE ub.blocker_id = $1
        ORDER BY ub.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(to_user_views(rows))
}

pub async fn list_muted(pool: &PgPool, user_id: Uuid) -> Result<Vec<FollowUserView>> {
    let rows: Vec<TargetRow> = sqlx::query_as(
        r#"
        SELECT u.id, u.display_name, u.email, um.created_at
        FROM user_mutes um
        JOIN users u ON u.id = um.muted_id
        WHERE um.muter_id = $1
        ORDER BY um.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(to_user_views(rows))
}
