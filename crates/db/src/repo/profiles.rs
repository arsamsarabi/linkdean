//! Profile access. Visibility is computed per request and is separate from
//! `category_access`, which only covers ownership and explicit shares.

use linkdean_core::{
    Bookmark, Error, FollowUserView, ProfileBookmarksPage, ProfileSummary, ProfileView, Result,
    User,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::blocks::is_blocked_either_way;
use super::categories::public_bookmark_view;
use super::follows::{follow_relation, is_following, list_followers, list_following};
use super::users::find_user_by_id;

/// Display name without the email local part that `user_share_label` can fall back to.
fn profile_label(user: &User) -> String {
    user.display_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("User")
        .to_string()
}

async fn can_view_profile(pool: &PgPool, viewer_id: Uuid, target: &User) -> Result<bool> {
    if viewer_id == target.id {
        return Ok(true);
    }
    if is_blocked_either_way(pool, viewer_id, target.id).await? {
        return Ok(false);
    }
    match target.profile_visibility.as_str() {
        "members" => Ok(true),
        "followers" => is_following(pool, viewer_id, target.id).await,
        _ => Ok(false),
    }
}

/// The target user, or `NotFound` for every denial so a hidden user's existence never shows.
pub async fn require_viewable_profile(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<User> {
    let target = find_user_by_id(pool, target_id)
        .await?
        .ok_or(Error::NotFound)?;
    if !can_view_profile(pool, viewer_id, &target).await? {
        return Err(Error::NotFound);
    }
    Ok(target)
}

pub async fn get_profile(pool: &PgPool, viewer_id: Uuid, target_id: Uuid) -> Result<ProfileView> {
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    let is_self = viewer_id == target.id;
    Ok(ProfileView {
        id: target.id,
        display_name: profile_label(&target),
        bio: target.bio.clone(),
        has_avatar: target.has_avatar,
        is_self,
        follow: follow_relation(pool, viewer_id, target.id).await?,
        email: is_self.then(|| target.email.clone()),
        profile_visibility: is_self.then(|| target.profile_visibility.clone()),
    })
}

const DEFAULT_PAGE_SIZE: i64 = 50;
const MAX_PAGE_SIZE: i64 = 100;

/// `$1` owner, `$2` category tiers the viewer may see. Only the owner's own categories
/// count, so a category shared by someone else cannot publish a bookmark.
const VISIBLE_BOOKMARK_FILTER: &str = r#"
    b.owner_user_id = $1
    AND b.deleted_at IS NULL
    AND b.is_vault = FALSE
    AND b.is_private = FALSE
    AND EXISTS (
        SELECT 1 FROM categories c
        WHERE c.owner_user_id = $1
          AND c.deleted_at IS NULL
          AND c.visibility = ANY($2)
          AND (
            c.id = b.category_id
            OR EXISTS (
                SELECT 1 FROM bookmark_categories bc
                WHERE bc.bookmark_id = b.id AND bc.category_id = c.id
            )
          )
    )
"#;

fn page_bounds(limit: Option<i64>, offset: Option<i64>) -> Result<(i64, i64)> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE);
    let offset = offset.unwrap_or(0);
    if !(1..=MAX_PAGE_SIZE).contains(&limit) {
        return Err(Error::BadRequest(format!(
            "limit must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    if offset < 0 {
        return Err(Error::BadRequest("offset must not be negative".into()));
    }
    Ok((limit, offset))
}

/// The owner previews everything others could see, so self counts as a follower.
async fn visible_tiers(pool: &PgPool, viewer_id: Uuid, target_id: Uuid) -> Result<Vec<String>> {
    let follower = viewer_id == target_id || is_following(pool, viewer_id, target_id).await?;
    let mut tiers = vec!["members".to_string()];
    if follower {
        tiers.push("followers".to_string());
    }
    Ok(tiers)
}

pub async fn profile_visible_bookmarks(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ProfileBookmarksPage> {
    let (limit, offset) = page_bounds(limit, offset)?;
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    let tiers = visible_tiers(pool, viewer_id, target.id).await?;

    let count_sql = format!("SELECT COUNT(*) FROM bookmarks b WHERE {VISIBLE_BOOKMARK_FILTER}");
    let (total,): (i64,) = sqlx::query_as(&count_sql)
        .bind(target.id)
        .bind(&tiers)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let rows_sql = format!(
        "SELECT b.* FROM bookmarks b WHERE {VISIBLE_BOOKMARK_FILTER} \
         ORDER BY b.updated_at DESC, b.id LIMIT $3 OFFSET $4"
    );
    let rows = sqlx::query_as::<_, Bookmark>(&rows_sql)
        .bind(target.id)
        .bind(&tiers)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    let mut bookmarks = Vec::with_capacity(rows.len());
    for b in rows {
        bookmarks.push(public_bookmark_view(pool, b).await?);
    }
    Ok(ProfileBookmarksPage { bookmarks, total })
}

async fn visible_summaries(
    pool: &PgPool,
    viewer_id: Uuid,
    rows: Vec<FollowUserView>,
) -> Result<Vec<ProfileSummary>> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(user) = find_user_by_id(pool, row.id).await? else {
            continue;
        };
        if can_view_profile(pool, viewer_id, &user).await? {
            out.push(ProfileSummary {
                id: user.id,
                display_name: profile_label(&user),
                has_avatar: user.has_avatar,
            });
        }
    }
    Ok(out)
}

pub async fn profile_followers(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<Vec<ProfileSummary>> {
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    visible_summaries(pool, viewer_id, list_followers(pool, target.id).await?).await
}

pub async fn profile_following(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<Vec<ProfileSummary>> {
    let target = require_viewable_profile(pool, viewer_id, target_id).await?;
    visible_summaries(pool, viewer_id, list_following(pool, target.id).await?).await
}
