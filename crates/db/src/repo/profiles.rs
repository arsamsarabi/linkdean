//! Profile access. Visibility is computed per request and is separate from
//! `category_access`, which only covers ownership and explicit shares.

use linkdean_core::{Error, ProfileView, Result, User};
use sqlx::PgPool;
use uuid::Uuid;

use super::blocks::is_blocked_either_way;
use super::follows::{follow_relation, is_following};
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
