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
