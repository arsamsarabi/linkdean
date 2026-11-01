use linkdean_core::crypto::{hash_password, verify_password};
use linkdean_core::{
    AdminUserSummary, Error, Result, UpdateAdminUser, UpdateProfile, UpdateSettings, User,
    UserPublic, UserSettings,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::moderation::{active_ban, active_ban_windows, count_active_admins_except, BanWindow};

pub async fn count_users(pool: &PgPool) -> Result<i64> {
    let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.0)
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password: Option<&str>,
    display_name: Option<&str>,
    oidc_subject: Option<&str>,
    oidc_issuer: Option<&str>,
) -> Result<User> {
    let password_hash = match password {
        Some(p) => {
            let p = p.to_string();
            Some(super::blocking::run_blocking(move || hash_password(&p)).await?)
        }
        None => None,
    };
    let is_first = count_users(pool).await? == 0;
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (
            email, display_name, password_hash, oidc_subject, oidc_issuer,
            is_admin, onboarding_completed
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(email.to_ascii_lowercase())
    .bind(display_name)
    .bind(password_hash)
    .bind(oidc_subject)
    .bind(oidc_issuer)
    .bind(is_first) // first user = admin
    .bind(false) // onboarding starts incomplete for new admins
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
            Error::Conflict("email already registered".into())
        } else {
            Error::msg(e.to_string())
        }
    })?;
    ensure_user_settings(pool, user.id).await?;
    Ok(user)
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email.to_ascii_lowercase())
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

/// Label for share badges: display_name, else email local-part, else `fallback`.
pub fn share_invitee_label(display_name: Option<&str>, email: &str, fallback: &str) -> String {
    if let Some(name) = display_name.map(str::trim).filter(|s| !s.is_empty()) {
        return name.to_string();
    }
    email
        .split('@')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

/// Label for share badges: display_name, else email local-part, else `"Owner"`.
pub fn user_share_label(user: &User) -> String {
    share_invitee_label(user.display_name.as_deref(), &user.email, "Owner")
}

/// Owner label for invitee views (`None` when viewer is the owner).
pub async fn owner_display_name_for_viewer(
    pool: &PgPool,
    owner_id: Uuid,
    viewer_id: Uuid,
) -> Result<Option<String>> {
    if owner_id == viewer_id {
        return Ok(None);
    }
    let Some(user) = find_user_by_id(pool, owner_id).await? else {
        return Ok(Some("Owner".into()));
    };
    Ok(Some(user_share_label(&user)))
}

pub async fn find_user_by_oidc(pool: &PgPool, issuer: &str, subject: &str) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE oidc_issuer = $1 AND oidc_subject = $2")
        .bind(issuer)
        .bind(subject)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn user_has_vault(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vaults WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.0 > 0)
}

pub async fn to_public(pool: &PgPool, user: &User) -> Result<UserPublic> {
    Ok(UserPublic {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        bio: user.bio.clone(),
        timezone: user.timezone.clone(),
        has_avatar: user.has_avatar,
        profile_visibility: user.profile_visibility.clone(),
        has_password: user.password_hash.is_some(),
        has_vault: user_has_vault(pool, user.id).await?,
        is_admin: user.is_admin,
        onboarding_completed: user.onboarding_completed,
        api_keys_enabled: user.api_keys_enabled,
    })
}

fn admin_summary(user: User, ban: Option<&BanWindow>) -> AdminUserSummary {
    AdminUserSummary {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        is_admin: user.is_admin,
        api_keys_enabled: user.api_keys_enabled,
        created_at: user.created_at,
        banned: ban.is_some(),
        banned_until: ban.and_then(|b| b.ends_at),
    }
}

pub async fn list_admin_users(pool: &PgPool) -> Result<Vec<AdminUserSummary>> {
    let rows = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let bans = active_ban_windows(pool).await?;
    Ok(rows
        .into_iter()
        .map(|u| {
            let ban = bans.iter().find(|b| b.user_id == u.id);
            admin_summary(u, ban)
        })
        .collect())
}

pub async fn update_admin_user(
    pool: &PgPool,
    user_id: Uuid,
    patch: UpdateAdminUser,
) -> Result<AdminUserSummary> {
    let Some(mut user) = find_user_by_id(pool, user_id).await? else {
        return Err(Error::NotFound);
    };
    if let Some(enabled) = patch.api_keys_enabled {
        user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET api_keys_enabled = $1, updated_at = now()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(enabled)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }
    if let Some(is_admin) = patch.is_admin {
        if !is_admin && user.is_admin {
            let others = count_active_admins_except(pool, user_id).await?;
            let banned = active_ban(pool, user_id).await?.is_some();
            if others == 0 && !banned {
                return Err(Error::BadRequest("cannot demote the last admin".into()));
            }
        }
        user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET is_admin = $1, updated_at = now()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(is_admin)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    }
    let ban = active_ban_windows(pool).await?;
    let ban = ban.iter().find(|b| b.user_id == user_id);
    Ok(admin_summary(user, ban))
}

pub async fn ensure_user_settings(pool: &PgPool, user_id: Uuid) -> Result<UserSettings> {
    sqlx::query_as::<_, UserSettings>(
        r#"
        INSERT INTO user_settings (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_user_settings(pool: &PgPool, user_id: Uuid) -> Result<UserSettings> {
    if let Some(s) =
        sqlx::query_as::<_, UserSettings>("SELECT * FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::msg(e.to_string()))?
    {
        return Ok(s);
    }
    ensure_user_settings(pool, user_id).await
}

pub async fn update_user_settings(
    pool: &PgPool,
    user_id: Uuid,
    patch: &UpdateSettings,
) -> Result<UserSettings> {
    let mut s = get_user_settings(pool, user_id).await?;
    if let Some(v) = patch.show_hidden {
        s.show_hidden = v;
    }
    if let Some(ref v) = patch.vault_search_mode {
        match v.as_str() {
            "never" | "when_unlocked" | "always" => s.vault_search_mode = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "vault_search_mode must be never|when_unlocked|always".into(),
                ))
            }
        }
    }
    if let Some(ref v) = patch.theme {
        match v.as_str() {
            "light" | "dark" | "system" => s.theme = v.clone(),
            _ => return Err(Error::BadRequest("theme must be light|dark|system".into())),
        }
    }
    if let Some(v) = patch.vault_auto_lock_minutes {
        if v < 0 {
            return Err(Error::BadRequest(
                "vault_auto_lock_minutes must be >= 0".into(),
            ));
        }
        s.vault_auto_lock_minutes = v;
    }
    if let Some(ref v) = patch.default_landing {
        match v.as_str() {
            "/" | "/library" => s.default_landing = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "default_landing must be /|/library".into(),
                ))
            }
        }
    }
    if let Some(ref v) = patch.library_view {
        match v.as_str() {
            "grid" | "explorer" => s.library_view = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "library_view must be grid|explorer".into(),
                ))
            }
        }
    }
    if let Some(v) = patch.trash_retention_days {
        if v <= 0 {
            return Err(Error::BadRequest("trash_retention_days must be > 0".into()));
        }
        s.trash_retention_days = v;
    }
    if let Some(v) = patch.show_deleted_in_library {
        s.show_deleted_in_library = v;
    }
    if let Some(v) = patch.show_vault_in_library {
        s.show_vault_in_library = v;
    }
    if let Some(ref v) = patch.vault_delete_mode {
        match v.as_str() {
            "trash" | "permanent" => s.vault_delete_mode = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "vault_delete_mode must be trash|permanent".into(),
                ))
            }
        }
    }
    sqlx::query_as::<_, UserSettings>(
        r#"
        UPDATE user_settings SET
            show_hidden = $2,
            vault_search_mode = $3,
            theme = $4,
            vault_auto_lock_minutes = $5,
            default_landing = $6,
            library_view = $7,
            trash_retention_days = $8,
            show_deleted_in_library = $9,
            show_vault_in_library = $10,
            vault_delete_mode = $11,
            updated_at = now()
        WHERE user_id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(s.show_hidden)
    .bind(&s.vault_search_mode)
    .bind(&s.theme)
    .bind(s.vault_auto_lock_minutes)
    .bind(&s.default_landing)
    .bind(&s.library_view)
    .bind(s.trash_retention_days)
    .bind(s.show_deleted_in_library)
    .bind(s.show_vault_in_library)
    .bind(&s.vault_delete_mode)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn update_profile(pool: &PgPool, user_id: Uuid, patch: &UpdateProfile) -> Result<User> {
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;

    let display_name = match &patch.display_name {
        Some(v) => v.clone(),
        None => user.display_name.clone(),
    };
    let bio = match &patch.bio {
        Some(v) => v.clone(),
        None => user.bio.clone(),
    };
    let timezone = match &patch.timezone {
        Some(v) => v.clone(),
        None => user.timezone.clone(),
    };
    let email = match &patch.email {
        Some(e) => {
            let e = e.trim().to_ascii_lowercase();
            if e.is_empty() {
                return Err(Error::BadRequest("email required".into()));
            }
            if e != user.email && find_user_by_email(pool, &e).await?.is_some() {
                return Err(Error::Conflict("email already registered".into()));
            }
            e
        }
        None => user.email.clone(),
    };

    let profile_visibility = patch
        .profile_visibility
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| user.profile_visibility.clone());

    sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET
            display_name = $2,
            bio = $3,
            timezone = $4,
            email = $5,
            profile_visibility = $6,
            updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(display_name)
    .bind(bio)
    .bind(timezone)
    .bind(email)
    .bind(profile_visibility)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn change_password(
    pool: &PgPool,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<()> {
    if new_password.len() < 8 {
        return Err(Error::BadRequest("password min 8 chars".into()));
    }
    let user = find_user_by_id(pool, user_id)
        .await?
        .ok_or(Error::NotFound)?;
    let hash = user
        .password_hash
        .as_deref()
        .ok_or_else(|| Error::BadRequest("account has no local password".into()))?;
    let hash = hash.to_string();
    let current = current_password.to_string();
    let ok = super::blocking::run_blocking(move || verify_password(&current, &hash)).await?;
    if !ok {
        return Err(Error::Unauthorized);
    }
    let new_password = new_password.to_string();
    let new_hash = super::blocking::run_blocking(move || hash_password(&new_password)).await?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .bind(new_hash)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn set_has_avatar(pool: &PgPool, user_id: Uuid, has_avatar: bool) -> Result<User> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users SET has_avatar = $2, updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(has_avatar)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn complete_onboarding(pool: &PgPool, user_id: Uuid) -> Result<User> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET onboarding_completed = TRUE, updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn authenticate_local(pool: &PgPool, email: &str, password: &str) -> Result<User> {
    let user = find_user_by_email(pool, email)
        .await?
        .ok_or(Error::Unauthorized)?;
    let hash = user.password_hash.clone().ok_or(Error::Unauthorized)?;
    let password = password.to_string();
    let ok = super::blocking::run_blocking(move || verify_password(&password, &hash)).await?;
    if !ok {
        return Err(Error::Unauthorized);
    }
    Ok(user)
}
