//! Organizations and teams membership (Phase A).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationRole {
    Owner,
    Admin,
    Member,
    Guest,
}

impl OrganizationRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Guest => "guest",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ()> {
        match s {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            _ => Err(()),
        }
    }

    /// Rank for permission checks (higher = more power).
    pub fn rank(self) -> u8 {
        match self {
            Self::Owner => 4,
            Self::Admin => 3,
            Self::Member => 2,
            Self::Guest => 1,
        }
    }

    pub fn at_least(self, min: Self) -> bool {
        self.rank() >= min.rank()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Maintainer,
    Editor,
    Viewer,
}

impl TeamRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Editor => "editor",
            Self::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ()> {
        match s {
            "maintainer" => Ok(Self::Maintainer),
            "editor" => Ok(Self::Editor),
            "viewer" => Ok(Self::Viewer),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct OrganizationMember {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Team {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub is_default: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workspace row for the switcher: an org the caller belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationWorkspace {
    pub id: Uuid,
    pub name: String,
    pub role: OrganizationRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationMemberView {
    pub user_id: Uuid,
    pub role: OrganizationRole,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TeamView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub is_default: bool,
    pub archived_at: Option<DateTime<Utc>>,
    /// Caller's team role when listed; org owners/admins act as maintainer without a row.
    pub role: Option<TeamRole>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganization {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateTeam {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct PatchOrganizationMember {
    pub role: OrganizationRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct OrganizationInvite {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub invitee_email: String,
    pub role: String,
    pub invited_by_user_id: Uuid,
    pub invitee_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claimed_by_user_id: Option<Uuid>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub rejected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationInviteTeamSpec {
    pub team_id: Uuid,
    #[serde(default)]
    pub role: Option<TeamRole>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationInvite {
    pub email: String,
    pub role: OrganizationRole,
    /// Teams to join on accept. Required for guests; optional for others (default team).
    #[serde(default)]
    pub teams: Vec<OrganizationInviteTeamSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationInviteView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub invitee_email: String,
    pub role: OrganizationRole,
    pub invited_by_user_id: Uuid,
    pub invitee_user_id: Option<Uuid>,
    pub team_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub rejected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationInviteInboxItem {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub role: OrganizationRole,
    pub inviter_display_name: String,
    pub created_at: DateTime<Utc>,
    pub rejected_at: Option<DateTime<Utc>>,
}
