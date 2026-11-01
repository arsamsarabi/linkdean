use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: Option<String>,
    pub oidc_subject: Option<String>,
    pub oidc_issuer: Option<String>,
    pub is_admin: bool,
    pub onboarding_completed: bool,
    pub api_keys_enabled: bool,
    pub bio: Option<String>,
    pub timezone: Option<String>,
    pub has_avatar: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub timezone: Option<String>,
    pub has_avatar: bool,
    pub has_password: bool,
    pub has_vault: bool,
    pub is_admin: bool,
    pub onboarding_completed: bool,
    pub api_keys_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub show_hidden: bool,
    pub vault_search_mode: String,
    pub theme: String,
    pub vault_auto_lock_minutes: i32,
    pub default_landing: String,
    pub library_view: String,
    pub trash_retention_days: i32,
    pub show_deleted_in_library: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfile {
    pub display_name: Option<Option<String>>,
    pub bio: Option<Option<String>>,
    pub timezone: Option<Option<String>>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub show_hidden: Option<bool>,
    pub vault_search_mode: Option<String>,
    pub theme: Option<String>,
    pub vault_auto_lock_minutes: Option<i32>,
    pub default_landing: Option<String>,
    pub library_view: Option<String>,
    pub trash_retention_days: Option<i32>,
    pub show_deleted_in_library: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlatformSettings {
    pub registration_open: bool,
    pub api_keys_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlatformSettings {
    pub registration_open: Option<bool>,
    pub api_keys_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_hash: String,
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Public key metadata (never includes the secret).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyView {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
}

/// One-shot create response: view + plaintext token (shown once).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub key: ApiKeyView,
    pub token: String,
}

/// Minimal admin account row (no private user data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserSummary {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub is_admin: bool,
    pub api_keys_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAdminUser {
    pub api_keys_enabled: Option<bool>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub is_hidden: bool,
    pub is_vault: bool,
    pub position: i32,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Effective access for the current user on a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryAccess {
    Owner,
    Read,
    Write,
}

impl CategoryAccess {
    pub fn can_write(self) -> bool {
        matches!(self, Self::Owner | Self::Write)
    }

    pub fn is_owner(self) -> bool {
        matches!(self, Self::Owner)
    }
}

/// Membership grant on a shared category (`read` / `write` only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareAccess {
    Read,
    Write,
}

impl ShareAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            _ => Err("access must be read or write".into()),
        }
    }

    pub fn to_category_access(self) -> CategoryAccess {
        match self {
            Self::Read => CategoryAccess::Read,
            Self::Write => CategoryAccess::Write,
        }
    }

    /// Same Owner/Read/Write mapping used for dashboards.
    pub fn to_dashboard_access(self) -> CategoryAccess {
        self.to_category_access()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryView {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub is_hidden: bool,
    pub is_vault: bool,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access: CategoryAccess,
    /// True when the owner has shared this folder (or an ancestor) with someone.
    #[serde(default)]
    pub shared_out: bool,
    /// Owner-only: invitee display labels for the outgoing share pill tooltip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
    /// Present for invitees — owner's display name (or email local-part).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl CategoryView {
    pub fn from_category(cat: Category, access: CategoryAccess) -> Self {
        Self {
            id: cat.id,
            owner_user_id: cat.owner_user_id,
            parent_id: cat.parent_id,
            name: cat.name,
            is_hidden: cat.is_hidden,
            is_vault: cat.is_vault,
            position: cat.position,
            created_at: cat.created_at,
            updated_at: cat.updated_at,
            access,
            shared_out: false,
            shared_with: Vec::new(),
            owner_display_name: None,
            deleted_at: cat.deleted_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryShare {
    pub id: Uuid,
    pub category_id: Uuid,
    pub invitee_user_id: Uuid,
    pub access: String,
    pub invited_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryShareView {
    pub id: Uuid,
    pub category_id: Uuid,
    pub invitee_user_id: Uuid,
    pub invitee_email: String,
    pub invitee_display_name: Option<String>,
    pub access: ShareAccess,
    pub invited_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryShare {
    pub email: String,
    pub access: ShareAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategoryShare {
    pub access: ShareAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DashboardShare {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub invitee_user_id: Uuid,
    pub access: String,
    pub invited_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardShareView {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub invitee_user_id: Uuid,
    pub invitee_email: String,
    pub invitee_display_name: Option<String>,
    pub access: ShareAccess,
    pub invited_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDashboardShare {
    pub email: String,
    pub access: ShareAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDashboardShare {
    pub access: ShareAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Bookmark {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub category_id: Option<Uuid>,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub favicon_url: Option<String>,
    pub preview_image_url: Option<String>,
    pub has_preview: bool,
    pub is_vault: bool,
    pub vault_ciphertext: Option<Vec<u8>>,
    pub vault_nonce: Option<Vec<u8>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRef {
    pub id: Uuid,
    pub name: String,
    /// Nested path, e.g. `Work / Side projects`.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkView {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    /// Primary / first category (legacy + export). Prefer `categories`.
    pub category_id: Option<Uuid>,
    pub categories: Vec<CategoryRef>,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub favicon_url: Option<String>,
    pub preview_image_url: Option<String>,
    pub has_preview: bool,
    pub is_vault: bool,
    pub tags: Vec<String>,
    /// Whether the requesting user may mutate this bookmark.
    pub can_write: bool,
    /// Owner has shared a folder this bookmark lives in (invitees see invitee badges instead).
    #[serde(default)]
    pub shared_out: bool,
    /// Owner-only: invitee display labels for the outgoing share pill tooltip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
    /// Present for invitees — owner's display name (or email local-part).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_display_name: Option<String>,
    /// Per-viewer flags (reading list / favorites / pin).
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_unread: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vault {
    pub user_id: Uuid,
    pub kdf_salt: Vec<u8>,
    pub wrapped_key: Vec<u8>,
    pub wrapped_nonce: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImportJob {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub source: String,
    pub status: String,
    pub total: i32,
    pub imported: i32,
    pub failed: i32,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultPayload {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateBookmark {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_unread: bool,
    #[serde(default)]
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateBookmark {
    pub url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub category_id: Option<Option<Uuid>>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tags: Option<Vec<String>>,
    pub is_favorite: Option<bool>,
    pub is_unread: Option<bool>,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub is_hidden: bool,
    pub is_vault: bool,
}

/// Copy a readable category tree (+ bookmarks) into a new owned root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkCategory {
    /// Defaults to `{name} (copy)`.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub parent_id: Option<Option<Uuid>>,
    pub is_hidden: Option<bool>,
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: UserPublic,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSetupRequest {
    pub passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultUnlockRequest {
    pub passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub favicon_url: Option<String>,
    pub preview_image_url: Option<String>,
    pub has_preview: bool,
    pub tags: Vec<String>,
    pub category_id: Option<Uuid>,
    pub is_vault: bool,
    pub can_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkdeanExport {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub categories: Vec<CategoryExport>,
    pub bookmarks: Vec<BookmarkExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryExport {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub is_hidden: bool,
    pub is_vault: bool,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkExport {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub category_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub is_vault: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Dashboard {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub title: String,
    pub is_default: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DashboardSection {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub title: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DashboardItem {
    pub id: Uuid,
    pub section_id: Uuid,
    pub bookmark_id: Uuid,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardItemView {
    pub id: Uuid,
    pub position: i32,
    pub bookmark: BookmarkView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSectionView {
    pub id: Uuid,
    pub title: String,
    pub position: i32,
    pub items: Vec<DashboardItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardView {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub title: String,
    pub is_default: bool,
    pub sections: Vec<DashboardSectionView>,
    pub access: CategoryAccess,
    /// True when the owner has shared this dashboard with someone.
    #[serde(default)]
    pub shared_out: bool,
    /// Owner-only: invitee display labels for the outgoing share pill tooltip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Lightweight row for switchers / settings (no sections).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub title: String,
    pub is_default: bool,
    pub access: CategoryAccess,
    #[serde(default)]
    pub shared_out: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashKind {
    Bookmark,
    Category,
    Dashboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashItem {
    pub kind: TrashKind,
    pub id: Uuid,
    pub title: String,
    pub deleted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurgeTrashRequest {
    pub items: Vec<TrashItemRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashItemRef {
    pub kind: TrashKind,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDashboard {
    pub title: Option<String>,
}

/// Rename and/or pin (`is_default: true` clears other pins for the owner).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchDashboard {
    pub title: Option<String>,
    pub is_default: Option<bool>,
}

/// Full layout replace for a personal dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutDashboardLayout {
    pub title: Option<String>,
    pub sections: Vec<PutDashboardSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutDashboardSection {
    pub title: String,
    pub bookmark_ids: Vec<Uuid>,
}

/// Shared-activity notification kinds (Milestone 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    ShareInvite,
    ShareAccessChanged,
    ShareRevoked,
    SharedEdit,
    SocialLike,
    SocialComment,
    SocialRating,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShareInvite => "share_invite",
            Self::ShareAccessChanged => "share_access_changed",
            Self::ShareRevoked => "share_revoked",
            Self::SharedEdit => "shared_edit",
            Self::SocialLike => "social_like",
            Self::SocialComment => "social_comment",
            Self::SocialRating => "social_rating",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "share_invite" => Ok(Self::ShareInvite),
            "share_access_changed" => Ok(Self::ShareAccessChanged),
            "share_revoked" => Ok(Self::ShareRevoked),
            "shared_edit" => Ok(Self::SharedEdit),
            "social_like" => Ok(Self::SocialLike),
            "social_comment" => Ok(Self::SocialComment),
            "social_rating" => Ok(Self::SocialRating),
            _ => Err(format!("unknown notification kind: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationSubjectType {
    Category,
    Dashboard,
    Bookmark,
}

impl NotificationSubjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Category => "category",
            Self::Dashboard => "dashboard",
            Self::Bookmark => "bookmark",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "category" => Ok(Self::Category),
            "dashboard" => Ok(Self::Dashboard),
            "bookmark" => Ok(Self::Bookmark),
            _ => Err(format!("unknown notification subject_type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub recipient_user_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub kind: String,
    pub subject_type: String,
    pub subject_id: Uuid,
    pub subject_title: String,
    pub body: String,
    pub payload: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationView {
    pub id: Uuid,
    pub kind: NotificationKind,
    pub subject_type: NotificationSubjectType,
    pub subject_id: Uuid,
    pub subject_title: String,
    pub body: String,
    pub payload: serde_json::Value,
    pub actor_user_id: Option<Uuid>,
    pub actor_display_name: Option<String>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Social subject (shared bookmark / list / dashboard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialSubjectType {
    Bookmark,
    Category,
    Dashboard,
}

impl SocialSubjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bookmark => "bookmark",
            Self::Category => "category",
            Self::Dashboard => "dashboard",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "bookmark" => Ok(Self::Bookmark),
            "category" => Ok(Self::Category),
            "dashboard" => Ok(Self::Dashboard),
            _ => Err(format!("unknown social subject_type: {s}")),
        }
    }

    pub fn to_notification_subject(self) -> NotificationSubjectType {
        match self {
            Self::Bookmark => NotificationSubjectType::Bookmark,
            Self::Category => NotificationSubjectType::Category,
            Self::Dashboard => NotificationSubjectType::Dashboard,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SocialComment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub subject_type: String,
    pub subject_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialCommentView {
    pub id: Uuid,
    pub user_id: Uuid,
    pub author_display_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSocialComment {
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSocialRating {
    pub rating: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialSummary {
    pub subject_type: SocialSubjectType,
    pub subject_id: Uuid,
    pub likes_count: i64,
    pub liked_by_me: bool,
    pub comments_count: i64,
    pub rating_avg: Option<f64>,
    pub rating_count: i64,
    pub my_rating: Option<i16>,
    pub comments: Vec<SocialCommentView>,
}
