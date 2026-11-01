use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserBan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub reason: String,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lifted_at: Option<DateTime<Utc>>,
    pub lifted_by: Option<Uuid>,
}

/// An active ban, as listed for admins.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BanView {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub is_admin: bool,
    pub reason: String,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_by_email: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// `ends_at` or `days` sets the end. Neither means permanent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateBan {
    pub reason: String,
    pub ends_at: Option<DateTime<Utc>>,
    pub days: Option<i64>,
}

/// Changes every listed ban. Fields left out keep their value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeBans {
    pub user_ids: Vec<Uuid>,
    pub reason: Option<String>,
    pub ends_at: Option<DateTime<Utc>>,
    pub days: Option<i64>,
    #[serde(default)]
    pub permanent: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiftBans {
    pub user_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModerationActionView {
    pub id: Uuid,
    pub user_id: Uuid,
    pub admin_id: Option<Uuid>,
    pub admin_email: Option<String>,
    pub action: String,
    pub subject_type: Option<String>,
    pub subject_id: Option<Uuid>,
    pub subject_title: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExposedCategory {
    pub id: Uuid,
    pub name: String,
    pub visibility: String,
    pub public_slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExposedBookmark {
    pub id: Uuid,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExposedShare {
    /// `category_share`, `dashboard_share` or `pending_share`.
    pub kind: String,
    pub id: Uuid,
    pub title: String,
    pub invitee: String,
    pub access: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExposedComment {
    pub id: Uuid,
    pub body: String,
    pub subject_type: String,
    pub created_at: DateTime<Utc>,
}

/// Everything a user shows to other people. Never private items or vault content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposedContent {
    pub public_links: Vec<ExposedCategory>,
    pub categories: Vec<ExposedCategory>,
    pub bookmarks: Vec<ExposedBookmark>,
    /// `None` when the profile is hidden.
    pub profile_visibility: Option<String>,
    pub shares: Vec<ExposedShare>,
    pub comments: Vec<ExposedComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerateAction {
    pub action: String,
    /// Needed for `removed_to_trash` and `share_revoked`.
    pub subject_type: Option<String>,
    pub subject_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerateRequest {
    pub reason: String,
    pub actions: Vec<ModerateAction>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModerateOutcome {
    pub applied: usize,
    pub skipped: usize,
    /// For the caller to drop from the search index.
    pub trashed_bookmark_ids: Vec<Uuid>,
}
