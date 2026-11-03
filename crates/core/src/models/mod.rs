use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod moderation;
pub use moderation::*;

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
    pub profile_visibility: String,
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
    pub profile_visibility: String,
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
    pub show_vault_in_library: bool,
    pub vault_delete_mode: String,
    pub updated_at: DateTime<Utc>,
}

/// Tells a field set to `null` from a missing one in a patch: a missing field stays `None`
/// through `serde(default)`, and `null` becomes `Some(None)`, which clears the value.
fn present_or_null<'de, T, D>(deserializer: D) -> std::result::Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfile {
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_name: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub bio: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub timezone: Option<Option<String>>,
    pub email: Option<String>,
    #[serde(default)]
    pub profile_visibility: Option<ProfileVisibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
    /// Also revoke every API key. Other login sessions always end.
    #[serde(default)]
    pub revoke_api_keys: bool,
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
    pub show_vault_in_library: Option<bool>,
    pub vault_delete_mode: Option<String>,
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
    pub banned: bool,
    /// End of the active ban. `None` while banned means permanent.
    pub banned_until: Option<DateTime<Utc>>,
    /// False for accounts that only sign in with SSO.
    pub has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAdminUser {
    pub api_keys_enabled: Option<bool>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationInvite {
    pub id: Uuid,
    pub code_hash: String,
    pub code_prefix: String,
    pub email: Option<String>,
    pub created_by_user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub used_by_user_id: Option<Uuid>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Public invite metadata (never includes the secret code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationInviteView {
    pub id: Uuid,
    pub code_prefix: String,
    pub email: Option<String>,
    pub created_by_user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub used_by_user_id: Option<Uuid>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegistrationInvite {
    /// Optional: bind invite to this email (case-insensitive match on register).
    pub email: Option<String>,
    /// Days until expiry (default 7, clamp 1..=90).
    pub expires_in_days: Option<i32>,
}

/// One-shot create response: view + plaintext code (shown once).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegistrationInviteResponse {
    pub invite: RegistrationInviteView,
    pub code: String,
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
    pub vault_id: Option<Uuid>,
    /// Optional icon key (e.g. `star`, `code`). `None` = default folder.
    pub icon: Option<String>,
    /// Unlisted public list slug (`ldl_…`). `None` = not published.
    pub public_slug: Option<String>,
    /// `private`, `followers` or `members`. Unrelated to `is_hidden`, which only affects the owner's views.
    pub visibility: String,
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

/// Who can open a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileVisibility {
    Members,
    Followers,
    Hidden,
}

impl ProfileVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Members => "members",
            Self::Followers => "followers",
            Self::Hidden => "hidden",
        }
    }
}

/// Who can see a category's bookmarks on the owner's profile. "Public" is reserved for
/// the no-login Public link, so the widest tier here is `Members`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryVisibility {
    Private,
    Followers,
    Members,
}

impl CategoryVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Followers => "followers",
            Self::Members => "members",
        }
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
    pub vault_id: Option<Uuid>,
    pub visibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
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
    /// When a live bookmark directly in this folder was last added or changed. Set by
    /// `list_categories` only; the sidebar orders its recent folders by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_bookmark_at: Option<DateTime<Utc>>,
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
            vault_id: cat.vault_id,
            visibility: cat.visibility,
            icon: cat.icon,
            position: cat.position,
            created_at: cat.created_at,
            updated_at: cat.updated_at,
            access,
            shared_out: false,
            shared_with: Vec::new(),
            owner_display_name: None,
            deleted_at: cat.deleted_at,
            last_bookmark_at: None,
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

/// Subject for a pending (pre-account) share invite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingShareSubjectType {
    Category,
    Dashboard,
}

impl PendingShareSubjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Category => "category",
            Self::Dashboard => "dashboard",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "category" => Ok(Self::Category),
            "dashboard" => Ok(Self::Dashboard),
            _ => Err("subject_type must be category or dashboard".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingShareInvite {
    pub id: Uuid,
    pub subject_type: String,
    pub subject_id: Uuid,
    pub invitee_email: String,
    pub access: String,
    pub invited_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claimed_by_user_id: Option<Uuid>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub invitee_user_id: Option<Uuid>,
    pub rejected_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub claim_token_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingShareInviteView {
    pub id: Uuid,
    pub subject_type: PendingShareSubjectType,
    pub subject_id: Uuid,
    pub invitee_email: String,
    pub access: ShareAccess,
    pub invited_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Single-use claim link token. Only present in the response that created it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_token: Option<String>,
}

/// An invite as its invitee sees it. `rejected_at` is set when they declined it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInviteInboxItem {
    pub id: Uuid,
    pub subject_type: PendingShareSubjectType,
    pub subject_id: Uuid,
    pub subject_title: String,
    pub access: ShareAccess,
    pub inviter_display_name: String,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySharesEnvelope {
    pub shares: Vec<CategoryShareView>,
    pub pending: Vec<PendingShareInviteView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreateCategoryShareOutcome {
    Active { share: CategoryShareView },
    Pending { invite: PendingShareInviteView },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSharesEnvelope {
    pub shares: Vec<DashboardShareView>,
    pub pending: Vec<PendingShareInviteView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CreateDashboardShareOutcome {
    Active { share: DashboardShareView },
    Pending { invite: PendingShareInviteView },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePendingShareInvite {
    pub access: ShareAccess,
}

/// Owner-facing public-link state for a category (list).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPublicLinkView {
    pub enabled: bool,
    /// Present when enabled. High-entropy unlisted slug.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// SPA path, e.g. `/l/ldl_…`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Anonymous read model for a published bookmark list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicListView {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub owner_display_name: String,
    /// The newest bookmarks, at most `PUBLIC_LIST_LIMIT` of them.
    pub bookmarks: Vec<PublicBookmarkView>,
    /// How many bookmarks the list holds, including any left out of `bookmarks`.
    #[serde(default)]
    pub total: i64,
}

/// Bookmark fields safe to expose on a public list (no notes / vault / owner ids).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicBookmarkView {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_image_url: Option<String>,
    pub has_preview: bool,
    /// Where anyone may load the stored preview, when `has_preview` and the caller is
    /// anonymous. Set on public lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    pub tags: Vec<String>,
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

/// Last probe result from the broken-link checker worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkCheckStatus {
    Ok,
    Broken,
    Unreachable,
    Unverifiable,
}

impl LinkCheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Broken => "broken",
            Self::Unreachable => "unreachable",
            Self::Unverifiable => "unverifiable",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "ok" => Ok(Self::Ok),
            "broken" => Ok(Self::Broken),
            "unreachable" => Ok(Self::Unreachable),
            "unverifiable" => Ok(Self::Unverifiable),
            _ => Err(format!("unknown link_check_status: {s}")),
        }
    }

    pub fn is_problem(self) -> bool {
        matches!(self, Self::Broken | Self::Unreachable)
    }
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
    pub vault_id: Option<Uuid>,
    pub is_private: bool,
    pub vault_ciphertext: Option<Vec<u8>>,
    pub vault_nonce: Option<Vec<u8>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub link_check_status: Option<String>,
    pub link_check_status_code: Option<i32>,
    pub link_check_error: Option<String>,
    pub link_checked_at: Option<DateTime<Utc>>,
    pub link_check_fail_count: i32,
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
    pub is_private: bool,
    pub is_vault: bool,
    pub vault_id: Option<Uuid>,
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
    /// `ok` / `broken` / `unreachable` / `unverifiable` — absent until first check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_check_status: Option<LinkCheckStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_check_status_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_check_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vault {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub kdf_salt: Vec<u8>,
    pub wrapped_key: Vec<u8>,
    pub wrapped_nonce: Vec<u8>,
    pub recovery_salt: Option<Vec<u8>>,
    pub recovery_wrapped_key: Option<Vec<u8>>,
    pub recovery_wrapped_nonce: Option<Vec<u8>>,
    pub global_wrapped_key: Option<Vec<u8>>,
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

/// Keys of the vaults a user has unlocked right now.
#[derive(Debug, Clone, Default)]
// Known issue: Keys are not zeroized when dropped, and this derives `Debug` over raw keys. Fix:
// `Zeroizing` keys, a redacted `Debug`, and an `Arc` around the ring.
pub struct VaultKeyring(std::collections::HashMap<Uuid, [u8; 32]>);

impl VaultKeyring {
    pub fn insert(&mut self, vault_id: Uuid, key: [u8; 32]) {
        self.0.insert(vault_id, key);
    }

    pub fn get(&self, vault_id: Uuid) -> Option<&[u8; 32]> {
        self.0.get(&vault_id)
    }

    pub fn contains(&self, vault_id: Uuid) -> bool {
        self.0.contains_key(&vault_id)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn vault_ids(&self) -> Vec<Uuid> {
        self.0.keys().copied().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultView {
    pub id: Uuid,
    pub name: String,
    pub unlocked: bool,
    pub has_recovery_key: bool,
    pub in_global: bool,
    /// Only for unlocked vaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVaultRequest {
    pub name: String,
    pub passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameVaultRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverVaultRequest {
    pub recovery_key: String,
    pub new_passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVaultRequest {
    pub name: String,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeGlobalPassphraseRequest {
    pub current_passphrase: String,
    pub new_passphrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveToVaultRequest {
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    #[serde(default)]
    pub bookmark_ids: Vec<Uuid>,
    pub destination_vault_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultMoveBlockerKind {
    Shared,
    PublicLink,
    OtherCategory,
    Dashboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMoveBlocker {
    pub kind: VaultMoveBlockerKind,
    pub subject_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum VaultMoveOutcome {
    Moved {
        left_search: Vec<Uuid>,
        joined_search: Vec<Uuid>,
    },
    Blocked(Vec<VaultMoveBlocker>),
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
    /// For a loose vault bookmark. Categories inside a vault imply their vault.
    #[serde(default)]
    pub vault_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateBookmark {
    pub url: Option<String>,
    pub title: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub notes: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub category_id: Option<Option<Uuid>>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tags: Option<Vec<String>>,
    /// Tags to add to whatever the bookmark has when the edit is saved.
    #[serde(default)]
    pub add_tags: Option<Vec<String>>,
    /// Folders to add the bookmark to, alongside the ones it is in when the edit is saved.
    #[serde(default)]
    pub add_category_ids: Option<Vec<Uuid>>,
    pub is_favorite: Option<bool>,
    pub is_unread: Option<bool>,
    pub is_pinned: Option<bool>,
    #[serde(default)]
    pub is_private: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub is_hidden: bool,
    #[serde(default)]
    pub vault_id: Option<Uuid>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// Copy a readable category tree (+ bookmarks) into a new owned root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkCategory {
    /// Defaults to `{name} (copy)`.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCategory {
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_id: Option<Option<Uuid>>,
    pub is_hidden: Option<bool>,
    pub position: Option<i32>,
    /// `Some(None)` clears the icon; `Some(Some(key))` sets it.
    #[serde(
        default,
        deserialize_with = "present_or_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<Option<String>>,
    #[serde(default)]
    pub visibility: Option<CategoryVisibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    /// Required when registration is closed (after bootstrap).
    #[serde(default)]
    pub invite_code: Option<String>,
    /// From a share invite's claim link.
    #[serde(default)]
    pub share_token: Option<String>,
    /// The server's `BOOTSTRAP_TOKEN`, needed for the first account when one is set.
    #[serde(default)]
    pub bootstrap_token: Option<String>,
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
    /// 1: categories and bookmarks with one category each. 2 adds every category of a
    /// bookmark, its flags and timestamps, and dashboards.
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub categories: Vec<CategoryExport>,
    pub bookmarks: Vec<BookmarkExport>,
    #[serde(default)]
    pub dashboards: Vec<DashboardExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardExport {
    pub title: String,
    #[serde(default)]
    pub is_default: bool,
    pub sections: Vec<DashboardSectionExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSectionExport {
    pub title: String,
    /// `BookmarkExport::id` values, in order.
    pub bookmark_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryExport {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub is_hidden: bool,
    pub is_vault: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub position: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookmarkExport {
    /// Lets dashboards in the same file point at this bookmark. Not kept on import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    /// First of `category_ids`, kept for version 1 readers.
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    pub tags: Vec<String>,
    pub is_vault: bool,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_unread: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl BookmarkExport {
    /// Every category this bookmark sits in, for files with or without `category_ids`.
    pub fn all_category_ids(&self) -> Vec<Uuid> {
        if self.category_ids.is_empty() {
            self.category_id.into_iter().collect()
        } else {
            self.category_ids.clone()
        }
    }
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

/// Rename and/or pin. Pinning a shared dashboard needs read access and only affects the caller.
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
    UserFollowed,
    ModerationChanges,
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
            Self::UserFollowed => "user_followed",
            Self::ModerationChanges => "moderation_changes",
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
            "user_followed" => Ok(Self::UserFollowed),
            "moderation_changes" => Ok(Self::ModerationChanges),
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
    User,
}

impl NotificationSubjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Category => "category",
            Self::Dashboard => "dashboard",
            Self::Bookmark => "bookmark",
            Self::User => "user",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "category" => Ok(Self::Category),
            "dashboard" => Ok(Self::Dashboard),
            "bookmark" => Ok(Self::Bookmark),
            "user" => Ok(Self::User),
            _ => Err(format!("unknown notification subject_type: {s}")),
        }
    }
}

/// Public follow-list entry (no email).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUserView {
    pub id: Uuid,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowRelation {
    pub following: bool,
    pub followers: i64,
    pub following_count: i64,
}

/// A user's profile as another signed-in user sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileView {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub has_avatar: bool,
    pub is_self: bool,
    pub follow: FollowRelation,
    /// Self only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Self only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_visibility: Option<String>,
}

/// One row of a follower or following list. Never carries an email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub id: Uuid,
    pub display_name: String,
    pub has_avatar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBookmarksPage {
    pub bookmarks: Vec<PublicBookmarkView>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRelation {
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteRelation {
    pub muted: bool,
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

/// Personal activity history actions (create / edit / delete / restore).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityAction {
    Created,
    Updated,
    Deleted,
    Restored,
}

impl ActivityAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
            Self::Restored => "restored",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "created" => Ok(Self::Created),
            "updated" => Ok(Self::Updated),
            "deleted" => Ok(Self::Deleted),
            "restored" => Ok(Self::Restored),
            _ => Err(format!("unknown activity action: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivitySubjectType {
    Bookmark,
    Category,
    Dashboard,
}

impl ActivitySubjectType {
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
            _ => Err(format!("unknown activity subject_type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: Uuid,
    pub action: ActivityAction,
    pub subject_type: ActivitySubjectType,
    pub subject_id: Uuid,
    pub subject_title: String,
    pub summary: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct ListActivityFilter {
    pub subject_type: Option<ActivitySubjectType>,
    pub action: Option<ActivityAction>,
    pub limit: i64,
}

#[cfg(test)]
mod patch_tests {
    use super::*;

    #[test]
    fn null_clears_and_a_missing_field_stays_unchanged() {
        let clear: UpdateCategory =
            serde_json::from_str(r#"{"parent_id":null,"icon":null}"#).unwrap();
        assert_eq!(clear.parent_id, Some(None));
        assert_eq!(clear.icon, Some(None));

        let keep: UpdateCategory = serde_json::from_str(r#"{"name":"x"}"#).unwrap();
        assert_eq!(keep.parent_id, None);
        assert_eq!(keep.icon, None);

        let profile: UpdateProfile = serde_json::from_str(r#"{"bio":null}"#).unwrap();
        assert_eq!(profile.bio, Some(None));
        assert_eq!(profile.display_name, None);

        let bookmark: UpdateBookmark =
            serde_json::from_str(r#"{"description":null,"title":"x"}"#).unwrap();
        assert_eq!(bookmark.description, Some(None));
        assert_eq!(bookmark.notes, None);
    }

    #[test]
    fn a_patch_survives_a_round_trip() {
        let patch: UpdateCategory = serde_json::from_str(r#"{"icon":null}"#).unwrap();
        let again: UpdateCategory =
            serde_json::from_str(&serde_json::to_string(&patch).unwrap()).unwrap();
        assert_eq!(again.icon, Some(None));
        assert_eq!(again.parent_id, None);
    }
}
