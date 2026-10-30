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
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSettings {
    pub user_id: Uuid,
    pub show_hidden: bool,
    pub vault_search_mode: String,
    pub theme: String,
    pub vault_auto_lock_minutes: i32,
    pub default_landing: String,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookmark {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub category_ids: Vec<Uuid>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBookmark {
    pub url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub category_id: Option<Option<Uuid>>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub is_hidden: bool,
    pub is_vault: bool,
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
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub favicon_url: Option<String>,
    pub preview_image_url: Option<String>,
    pub has_preview: bool,
    pub tags: Vec<String>,
    pub category_id: Option<Uuid>,
    pub is_vault: bool,
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
