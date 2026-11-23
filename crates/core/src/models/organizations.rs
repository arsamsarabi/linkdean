//! Organizations and teams membership (Phase A) + invite links (Phase B).

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

    /// Rank for permission checks (higher = more power).
    pub fn rank(self) -> u8 {
        match self {
            Self::Maintainer => 3,
            Self::Editor => 2,
            Self::Viewer => 1,
        }
    }

    pub fn at_least(self, min: Self) -> bool {
        self.rank() >= min.rank()
    }

    /// Map team role onto folder/bookmark access used elsewhere in the app.
    pub fn to_category_access(self) -> crate::CategoryAccess {
        match self {
            Self::Maintainer => crate::CategoryAccess::Owner,
            Self::Editor => crate::CategoryAccess::Write,
            Self::Viewer => crate::CategoryAccess::Read,
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
    pub invite_links_enabled: bool,
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
    pub deleted_at: Option<DateTime<Utc>>,
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
    /// Set while the org is in the 30-day delete grace (owners only see these).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    /// When false, invite-link create/accept is refused (default).
    pub invite_links_enabled: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    /// Caller's team role when listed; org owners/admins act as maintainer without a row.
    pub role: Option<TeamRole>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ConfirmOrganizationName {
    /// Must match the organization name exactly (trimmed).
    pub confirm_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct OrganizationAuditEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationAuditEventView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_email: Option<String>,
    pub action: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
pub struct ListOrganizationAuditQuery {
    #[serde(default)]
    pub actor_user_id: Option<Uuid>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct OrganizationInviteLink {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub code_hash: String,
    pub code_prefix: String,
    pub role: String,
    pub created_by_user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub max_uses: Option<i32>,
    pub use_count: i32,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationInviteLink {
    pub role: OrganizationRole,
    /// Days until expiry; clamped 1..=90; default 7.
    #[serde(default)]
    pub expires_in_days: Option<u32>,
    /// Max redemptions. `None` or omit = unlimited.
    #[serde(default)]
    pub max_uses: Option<i32>,
    /// Teams to join on accept. Required for guests; optional for others (default team).
    #[serde(default)]
    pub teams: Vec<OrganizationInviteTeamSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationInviteLinkView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub code_prefix: String,
    pub role: OrganizationRole,
    pub created_by_user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub max_uses: Option<i32>,
    pub use_count: i32,
    pub team_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationInviteLinkResponse {
    pub link: OrganizationInviteLinkView,
    /// Full secret code; returned once on create.
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct AcceptOrganizationInviteLink {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct PatchOrganizationInviteLinks {
    pub invite_links_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DomainJoinMode {
    AutoJoin,
    Request,
}

impl DomainJoinMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AutoJoin => "auto_join",
            Self::Request => "request",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ()> {
        match s {
            "auto_join" => Ok(Self::AutoJoin),
            "request" => Ok(Self::Request),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct OrganizationDomain {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub domain: String,
    pub verification_token: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub join_mode: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationDomainView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub domain: String,
    /// DNS name to publish (e.g. `_linkdean-challenge.acme.com`).
    pub txt_name: String,
    /// Exact TXT value to publish.
    pub txt_value: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub join_mode: DomainJoinMode,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationDomain {
    pub domain: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct PatchOrganizationDomain {
    pub join_mode: DomainJoinMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationDomainSuggestion {
    pub organization_id: Uuid,
    pub organization_name: String,
    pub domain: String,
    pub join_mode: DomainJoinMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct OrganizationDomainJoinRequest {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub domain_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by_user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationDomainJoinRequestView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub domain_id: Uuid,
    pub domain: String,
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DomainJoinResponse {
    /// `joined` or `pending`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<OrganizationWorkspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationSsoConfig {
    pub organization_id: Uuid,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret_ciphertext: Vec<u8>,
    pub client_secret_nonce: Vec<u8>,
    pub display_name: String,
    pub enabled: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationSsoConfigView {
    pub organization_id: Uuid,
    pub issuer_url: String,
    pub client_id: String,
    pub display_name: String,
    pub enabled: bool,
    /// Redirect URI to register at the IdP (`{APP_URL}/api/auth/org-oidc/callback`).
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpsertOrganizationSso {
    pub issuer_url: String,
    pub client_id: String,
    /// Required on create; omit on update to keep the stored secret.
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct DiscoverOrgSsoRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DiscoverOrgSsoResponse {
    pub sso: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationScimToken {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub token_hash: String,
    pub prefix: String,
    pub created_by_user_id: Uuid,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OrganizationScimTokenView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationScimToken {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationScimTokenResponse {
    pub token: String,
    pub view: OrganizationScimTokenView,
}
