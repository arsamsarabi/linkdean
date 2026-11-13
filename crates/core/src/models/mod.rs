use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod abuse;
pub use abuse::*;
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
    /// When the account proved it controls `email`. `None` until then.
    pub email_verified_at: Option<DateTime<Utc>>,
    /// When the account is purged after a self-serve delete request. `None` when not scheduled.
    pub deletion_scheduled_at: Option<DateTime<Utc>>,
}

impl User {
    /// The issuer host of the attached single sign-on identity, if any.
    pub fn sso_provider(&self) -> Option<String> {
        match (&self.oidc_issuer, &self.oidc_subject) {
            (Some(issuer), Some(_)) => Some(issuer_host(issuer)),
            _ => None,
        }
    }
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
    pub email_verified: bool,
    /// Whether a single sign-on identity is attached.
    pub sso_connected: bool,
    /// The host of the attached identity's issuer, for display.
    pub sso_provider: Option<String>,
    /// Set while the account is scheduled for deletion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_scheduled_at: Option<DateTime<Utc>>,
}

/// Body of `POST /users/me/deletion`.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleAccountDeletion {
    /// Must match the account email (case-insensitive).
    pub confirm_email: String,
    /// Required for accounts with a local password; ignored for single sign-on only accounts.
    #[serde(default)]
    pub password: Option<String>,
}

/// The host of an OIDC issuer URL, for display: `https://id.example.com/realms/x` →
/// `id.example.com`. Falls back to the issuer itself when it does not parse.
pub fn issuer_host(issuer: &str) -> String {
    url::Url::parse(issuer)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| issuer.to_string())
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
    /// Accepted only when it matches the current address. An address changes through an
    /// emailed confirmation link, or by an instance admin.
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
    pub require_verified_email: bool,
    /// Max owned bookmarks (non-deleted). `None` = unlimited.
    pub max_bookmarks: Option<i64>,
    /// Max bytes for avatar + preview assets. `None` = unlimited.
    pub max_stored_image_bytes: Option<i64>,
    /// Max enrich actions (page/favicon/OG/screenshot) per UTC day. `None` = unlimited.
    pub max_enrich_actions_per_day: Option<i64>,
    /// Max import payload bytes (clamped to the hard 10 MiB cap). `None` = hard cap only.
    pub max_import_bytes: Option<i64>,
    /// Max uploaded image body bytes (clamped to avatar/preview hard caps). `None` = hard caps only.
    pub max_upload_image_bytes: Option<i64>,
    /// Where to send abuse and takedown reports. `None` until an admin sets it.
    pub abuse_contact_email: Option<String>,
    /// Hours between manual retries of failed screenshots.
    pub screenshot_retry_cooldown_hours: i32,
    /// Failed screenshots older than this many days are retried automatically.
    pub screenshot_auto_retry_days: i32,
    /// When an admin last retried failed screenshots by hand.
    pub screenshot_retry_last_run_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Effective quotas for a user. Today = platform row; later may AND with a plan.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ResolvedLimits {
    pub max_bookmarks: Option<i64>,
    pub max_stored_image_bytes: Option<i64>,
    pub max_enrich_actions_per_day: Option<i64>,
    pub max_import_bytes: Option<i64>,
    pub max_upload_image_bytes: Option<i64>,
}

impl From<&PlatformSettings> for ResolvedLimits {
    fn from(p: &PlatformSettings) -> Self {
        Self {
            max_bookmarks: p.max_bookmarks,
            max_stored_image_bytes: p.max_stored_image_bytes,
            max_enrich_actions_per_day: p.max_enrich_actions_per_day,
            max_import_bytes: p.max_import_bytes,
            max_upload_image_bytes: p.max_upload_image_bytes,
        }
    }
}

/// Usage snapshot for Settings. Limits mirror [`ResolvedLimits`]; used gauges/counters are filled in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUsageView {
    pub limits: ResolvedLimits,
    pub bookmarks: i64,
    pub stored_image_bytes: i64,
    pub enrich_actions_today: i64,
    pub enrich_period_start: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlatformSettings {
    pub registration_open: Option<bool>,
    pub api_keys_enabled: Option<bool>,
    pub require_verified_email: Option<bool>,
    /// `Some(None)` clears; `Some(Some(n))` sets; omit = no change.
    #[serde(default, deserialize_with = "deserialize_option_option_i64")]
    pub max_bookmarks: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_option_option_i64")]
    pub max_stored_image_bytes: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_option_option_i64")]
    pub max_enrich_actions_per_day: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_option_option_i64")]
    pub max_import_bytes: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_option_option_i64")]
    pub max_upload_image_bytes: Option<Option<i64>>,
    /// `Some(None)` clears; `Some(Some(email))` sets; omit = no change.
    #[serde(default, deserialize_with = "deserialize_option_option_string")]
    pub abuse_contact_email: Option<Option<String>>,
    /// 1 to 720.
    pub screenshot_retry_cooldown_hours: Option<i32>,
    /// 1 to 90.
    pub screenshot_auto_retry_days: Option<i32>,
}

fn deserialize_option_option_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(deserializer)?))
}

fn deserialize_option_option_i64<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<i64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<i64>::deserialize(deserializer)?))
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
    pub email_verified: bool,
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
    #[serde(skip)]
    pub email_claim_token_hash: Option<String>,
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
    /// Cloudflare Turnstile widget token; required when the server has Turnstile keys.
    #[serde(default)]
    pub turnstile_token: Option<String>,
    /// Terms of Service version the user accepted; must equal `legal::TERMS_VERSION`.
    #[serde(default)]
    pub terms_version: String,
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
    #[serde(default)]
    pub bookmark_ids: Vec<Uuid>,
    #[serde(flatten)]
    pub settings: DashboardSectionSettings,
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
    pub kind: String,
    pub display: String,
    pub item_limit: i32,
    pub source_category_id: Option<Uuid>,
    pub source_tag: Option<String>,
    pub source_site: Option<String>,
    pub source_favorites: bool,
    pub source_unread: bool,
    pub open_window: String,
    pub width: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DashboardSection {
    /// The stored settings; unknown values (never written by this version) fall back to defaults.
    pub fn settings(&self) -> DashboardSectionSettings {
        DashboardSectionSettings {
            kind: DashboardSectionKind::parse(&self.kind).unwrap_or_default(),
            display: DashboardSectionDisplay::parse(&self.display).unwrap_or_default(),
            item_limit: self.item_limit,
            source: DashboardSectionSource {
                category_id: self.source_category_id,
                tag: self.source_tag.clone(),
                site: self.source_site.clone(),
                favorites: self.source_favorites,
                unread: self.source_unread,
            },
            open_window: DashboardOpenWindow::parse(&self.open_window).unwrap_or_default(),
            width: DashboardSectionWidth::parse(&self.width).unwrap_or_default(),
        }
    }
}

/// What fills a dashboard section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardSectionKind {
    /// Bookmarks pinned by hand.
    #[default]
    Pinned,
    /// The newest bookmarks matching the section's source.
    Latest,
    /// The most opened bookmarks matching the section's source.
    MostOpened,
    /// The bookmarks the owner opened last. Never on a shared dashboard.
    RecentlyVisited,
}

impl DashboardSectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Latest => "latest",
            Self::MostOpened => "most_opened",
            Self::RecentlyVisited => "recently_visited",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pinned" => Self::Pinned,
            "latest" => Self::Latest,
            "most_opened" => Self::MostOpened,
            "recently_visited" => Self::RecentlyVisited,
            _ => return None,
        })
    }

    /// Sections that fill themselves from the owner's library, so never on a shared dashboard.
    pub fn fills_itself(self) -> bool {
        self != Self::Pinned
    }
}

/// How a section shows its bookmarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardSectionDisplay {
    Small,
    #[default]
    Medium,
    Large,
    Icons,
}

impl DashboardSectionDisplay {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Icons => "icons",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "small" => Self::Small,
            "medium" => Self::Medium,
            "large" => Self::Large,
            "icons" => Self::Icons,
            _ => return None,
        })
    }
}

/// How much of the board's width a section takes. Phones always show full width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardSectionWidth {
    #[default]
    Full,
    Half,
    Third,
}

impl DashboardSectionWidth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Half => "half",
            Self::Third => "third",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "full" => Self::Full,
            "half" => Self::Half,
            "third" => Self::Third,
            _ => return None,
        })
    }
}

/// The period "Most opened" counts opens over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardOpenWindow {
    #[serde(rename = "last_30_days")]
    Last30Days,
    #[default]
    AllTime,
}

impl DashboardOpenWindow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Last30Days => "last_30_days",
            Self::AllTime => "all_time",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "last_30_days" => Self::Last30Days,
            "all_time" => Self::AllTime,
            _ => return None,
        })
    }
}

/// Filters for a section that fills itself. A bookmark must match every one that is set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardSectionSource {
    /// A folder of the owner's, with its subfolders.
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub tag: Option<String>,
    /// A website, such as `github.com`; its subdomains match too.
    #[serde(default)]
    pub site: Option<String>,
    #[serde(default)]
    pub favorites: bool,
    #[serde(default)]
    pub unread: bool,
}

pub const DASHBOARD_SECTION_MIN_ITEMS: i32 = 4;
pub const DASHBOARD_SECTION_MAX_ITEMS: i32 = 24;
pub const DASHBOARD_SECTION_DEFAULT_ITEMS: i32 = 8;

fn default_section_item_limit() -> i32 {
    DASHBOARD_SECTION_DEFAULT_ITEMS
}

/// A section's type, display mode and source. Every field has a default, so older layouts
/// and exports (title and pins only) still read as pinned, medium sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardSectionSettings {
    #[serde(default)]
    pub kind: DashboardSectionKind,
    #[serde(default)]
    pub display: DashboardSectionDisplay,
    #[serde(default = "default_section_item_limit")]
    pub item_limit: i32,
    #[serde(default)]
    pub source: DashboardSectionSource,
    #[serde(default)]
    pub open_window: DashboardOpenWindow,
    #[serde(default)]
    pub width: DashboardSectionWidth,
}

impl Default for DashboardSectionSettings {
    fn default() -> Self {
        Self {
            kind: DashboardSectionKind::default(),
            display: DashboardSectionDisplay::default(),
            item_limit: DASHBOARD_SECTION_DEFAULT_ITEMS,
            source: DashboardSectionSource::default(),
            open_window: DashboardOpenWindow::default(),
            width: DashboardSectionWidth::default(),
        }
    }
}

impl DashboardSectionSettings {
    /// Trims the source, drops empty values, and checks the bookmark count.
    pub fn normalized(mut self) -> crate::Result<Self> {
        if !(DASHBOARD_SECTION_MIN_ITEMS..=DASHBOARD_SECTION_MAX_ITEMS).contains(&self.item_limit) {
            return Err(crate::Error::BadRequest(format!(
                "a section shows {DASHBOARD_SECTION_MIN_ITEMS} to {DASHBOARD_SECTION_MAX_ITEMS} bookmarks"
            )));
        }
        let clean = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.source.tag = clean(self.source.tag);
        self.source.site = clean(self.source.site).map(|s| normalize_site(&s));
        if self.source.site.as_deref() == Some("") {
            self.source.site = None;
        }
        if !self.kind.fills_itself() || self.kind == DashboardSectionKind::RecentlyVisited {
            // Pinned and Recently visited have no source.
            self.source = DashboardSectionSource::default();
        }
        Ok(self)
    }
}

/// `https://www.GitHub.com/x` and `github.com` both become `github.com`.
pub fn normalize_site(input: &str) -> String {
    let s = input.trim().to_ascii_lowercase();
    let s = s.split_once("://").map(|(_, rest)| rest).unwrap_or(&s);
    let host = s.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit_once('@').map(|(_, h)| h).unwrap_or(host);
    let host = host.split(':').next().unwrap_or("");
    host.strip_prefix("www.")
        .unwrap_or(host)
        .trim_matches('.')
        .to_string()
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
    /// How often the viewer opened it: in the last 30 days for a section counting that
    /// window, otherwise of all time. Absent when never opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_count: Option<i64>,
    /// When the viewer last opened it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSectionView {
    pub id: Uuid,
    pub title: String,
    pub position: i32,
    #[serde(flatten)]
    pub settings: DashboardSectionSettings,
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
    /// Pins, for a pinned section. Ignored for sections that fill themselves.
    #[serde(default)]
    pub bookmark_ids: Vec<Uuid>,
    #[serde(flatten)]
    pub settings: DashboardSectionSettings,
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
    /// Every kind, in the order settings lists them.
    pub const ALL: [Self; 9] = [
        Self::ShareInvite,
        Self::ShareAccessChanged,
        Self::ShareRevoked,
        Self::SharedEdit,
        Self::SocialLike,
        Self::SocialComment,
        Self::SocialRating,
        Self::UserFollowed,
        Self::ModerationChanges,
    ];

    /// Whether this kind is emailed when the user has not chosen.
    pub fn email_by_default(self) -> bool {
        matches!(self, Self::ShareInvite | Self::ModerationChanges)
    }

    /// What settings and the email footer call this kind.
    pub fn label(self) -> &'static str {
        match self {
            Self::ShareInvite => "Share invites",
            Self::ShareAccessChanged => "Share access changes",
            Self::ShareRevoked => "Revoked shares",
            Self::SharedEdit => "Edits in shared folders",
            Self::SocialLike => "Likes",
            Self::SocialComment => "Comments",
            Self::SocialRating => "Ratings",
            Self::UserFollowed => "New followers",
            Self::ModerationChanges => "Moderation changes",
        }
    }

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

#[cfg(test)]
mod issuer_host_tests {
    use super::issuer_host;

    #[test]
    fn keeps_only_the_host() {
        assert_eq!(
            issuer_host("https://id.example.com/realms/links"),
            "id.example.com"
        );
        assert_eq!(issuer_host("http://127.0.0.1:18090/default"), "127.0.0.1");
        assert_eq!(issuer_host("not a url"), "not a url");
    }
}

#[cfg(test)]
mod notification_kind_tests {
    use super::NotificationKind;

    #[test]
    fn every_notification_kind_is_listed_once() {
        // A new variant fails to compile here until it is given a place in `ALL`.
        fn position(k: NotificationKind) -> usize {
            match k {
                NotificationKind::ShareInvite => 0,
                NotificationKind::ShareAccessChanged => 1,
                NotificationKind::ShareRevoked => 2,
                NotificationKind::SharedEdit => 3,
                NotificationKind::SocialLike => 4,
                NotificationKind::SocialComment => 5,
                NotificationKind::SocialRating => 6,
                NotificationKind::UserFollowed => 7,
                NotificationKind::ModerationChanges => 8,
            }
        }
        for (i, k) in NotificationKind::ALL.iter().enumerate() {
            assert_eq!(position(*k), i);
            assert_eq!(NotificationKind::parse(k.as_str()), Ok(*k));
            assert!(!k.label().is_empty());
        }
    }
}
