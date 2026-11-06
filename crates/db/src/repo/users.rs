use linkdean_core::crypto::{
    hash_password, password_needs_rehash, verify_dummy_password, verify_password, MIN_SECRET_LEN,
};
use linkdean_core::{
    AdminUserSummary, Error, Result, UpdateAdminUser, UpdateProfile, UpdateSettings, User,
    UserPublic, UserSettings,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::account_tokens::TokenPurpose;
use super::moderation::{active_ban_windows, BanWindow};

pub async fn count_users(pool: &PgPool) -> Result<i64> {
    let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(n.0)
}

/// Who may sign up, checked inside the sign-up transaction.
#[derive(Debug, Default, Clone, Copy)]
pub struct SignupGate<'a> {
    /// After the first account, require open registration or a valid invite. Off for tests and
    /// tools that create users directly.
    pub enforce: bool,
    /// Claimed in the same transaction as the insert, so one invite makes one account.
    pub invite_code: Option<&'a str>,
    /// `None` when no `BOOTSTRAP_TOKEN` is set; otherwise whether the request presented it. Only
    /// the first account, which becomes admin, needs it.
    pub bootstrap_token_ok: Option<bool>,
}

/// Creates a user with no sign-up checks. The first user still becomes admin.
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password: Option<&str>,
    display_name: Option<&str>,
    oidc_subject: Option<&str>,
    oidc_issuer: Option<&str>,
) -> Result<User> {
    create_user_gated(
        pool,
        email,
        password,
        display_name,
        oidc_subject,
        oidc_issuer,
        SignupGate::default(),
    )
    .await
}

/// Creates a user under `gate`. Sign-ups are serialised by an advisory lock, so only one
/// account can see an empty instance and become admin, and the invite is claimed in the same
/// transaction as the insert.
pub async fn create_user_gated(
    pool: &PgPool,
    email: &str,
    password: Option<&str>,
    display_name: Option<&str>,
    oidc_subject: Option<&str>,
    oidc_issuer: Option<&str>,
    gate: SignupGate<'_>,
) -> Result<User> {
    let password_hash = match password {
        Some(p) => {
            let p = p.to_string();
            Some(super::blocking::run_kdf(move || hash_password(&p)).await?)
        }
        None => None,
    };
    let email = email.trim().to_ascii_lowercase();
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('user_signup'))")
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let is_first = count == 0;
    if is_first {
        if gate.bootstrap_token_ok == Some(false) {
            return Err(Error::Forbidden);
        }
    } else if gate.enforce && gate.invite_code.is_none() {
        let (open,): (bool,) =
            sqlx::query_as("SELECT registration_open FROM platform_settings WHERE id = TRUE")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| Error::msg(e.to_string()))?;
        if !open {
            return Err(Error::Forbidden);
        }
    }
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
    .bind(&email)
    .bind(display_name)
    .bind(password_hash)
    .bind(oidc_subject)
    .bind(oidc_issuer)
    .bind(is_first) // first user = admin
    .bind(false) // onboarding starts incomplete for new admins
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
            Error::Conflict("email already registered".into())
        } else {
            Error::msg(e.to_string())
        }
    })?;
    if gate.enforce && !is_first {
        if let Some(code) = gate.invite_code {
            super::invites::claim_registration_invite(&mut tx, code, &email, user.id).await?;
        }
    }
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    ensure_user_settings(pool, user.id).await?;
    Ok(user)
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email.trim().to_ascii_lowercase())
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

/// Label for people who were never given this user's email: anonymous visitors, followers,
/// other commenters. The display name, else `fallback`. Never any part of the email.
pub fn public_user_label(user: &User, fallback: &str) -> String {
    user.display_name
        .as_deref()
        .map(str::trim)
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
        email_verified: user.email_verified_at.is_some(),
        sso_connected: user.oidc_subject.is_some(),
        sso_provider: user.sso_provider(),
        deletion_scheduled_at: user.deletion_scheduled_at,
    })
}

fn admin_summary(user: User, ban: Option<&BanWindow>) -> AdminUserSummary {
    AdminUserSummary {
        id: user.id,
        has_password: user.password_hash.is_some(),
        email_verified: user.email_verified_at.is_some(),
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

/// Admin edits to another account. Demotion checks for another active admin and updates under
/// the `user_bans` advisory lock that bans also take, so two admins demoting each other (or a
/// ban racing a demotion) cannot leave the instance without one.
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
        let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('user_bans'))")
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        let (was_admin, banned): (bool, bool) = sqlx::query_as(
            r#"
            SELECT u.is_admin, EXISTS (
                SELECT 1 FROM user_bans b
                WHERE b.user_id = u.id
                  AND b.lifted_at IS NULL
                  AND (b.ends_at IS NULL OR b.ends_at > now())
            )
            FROM users u WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)?;
        // A banned admin is not an active one, so demoting them never removes the last.
        if !is_admin
            && was_admin
            && !banned
            && super::moderation::active_admins_except(&mut tx, user_id).await? == 0
        {
            return Err(Error::BadRequest("cannot demote the last admin".into()));
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
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    }
    let ban = active_ban_windows(pool).await?;
    let ban = ban.iter().find(|b| b.user_id == user_id);
    Ok(admin_summary(user, ban))
}

/// Matches the `CHECK` on `user_settings.trash_retention_days`.
const MAX_TRASH_RETENTION_DAYS: i32 = 3650;

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
            "grid" | "list" | "explorer" => s.library_view = v.clone(),
            _ => {
                return Err(Error::BadRequest(
                    "library_view must be grid|list|explorer".into(),
                ))
            }
        }
    }
    if let Some(v) = patch.trash_retention_days {
        if !(1..=MAX_TRASH_RETENTION_DAYS).contains(&v) {
            return Err(Error::BadRequest(format!(
                "trash_retention_days must be 1 to {MAX_TRASH_RETENTION_DAYS}"
            )));
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
    // The address changes only through a confirmation link (or an admin), so an unverified
    // address can never take over invites. Sending the current one back is fine.
    if let Some(e) = &patch.email {
        if e.trim().to_ascii_lowercase() != user.email {
            return Err(Error::BadRequest(
                "change your email from Settings → Account".into(),
            ));
        }
    }

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
            profile_visibility = $5,
            updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(display_name)
    .bind(bio)
    .bind(timezone)
    .bind(profile_visibility)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

/// Sets a new password and ends every other login session, keeping the one whose token hash
/// is `keep_session_hash`. With `revoke_api_keys`, every API key is revoked too.
/// Returns how many sessions ended and how many keys were revoked.
pub async fn change_password(
    pool: &PgPool,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
    keep_session_hash: Option<&str>,
    revoke_api_keys: bool,
) -> Result<(u64, u64)> {
    if new_password.len() < MIN_SECRET_LEN {
        return Err(Error::BadRequest(format!(
            "password min {MIN_SECRET_LEN} chars"
        )));
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
    let ok = super::blocking::run_kdf(move || verify_password(&current, &hash)).await?;
    if !ok {
        return Err(Error::Unauthorized);
    }
    let new_password = new_password.to_string();
    let new_hash = super::blocking::run_kdf(move || hash_password(&new_password)).await?;
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .bind(new_hash)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let sessions =
        sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash IS DISTINCT FROM $2")
            .bind(user_id)
            .bind(keep_session_hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::msg(e.to_string()))?
            .rows_affected();
    let keys = if revoke_api_keys {
        sqlx::query(
            "UPDATE api_keys SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .rows_affected()
    } else {
        0
    };
    super::account_tokens::void_account_tokens(&mut tx, user_id, TokenPurpose::PasswordReset)
        .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok((sessions, keys))
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
    let user = find_user_by_email(pool, email).await?;
    let Some((user, hash)) = user.and_then(|u| u.password_hash.clone().map(|h| (u, h))) else {
        // No account, or no local password: spend the same time a real check takes.
        let password = password.to_string();
        super::blocking::run_kdf(move || {
            verify_dummy_password(&password);
            Ok(())
        })
        .await?;
        return Err(Error::Unauthorized);
    };
    let pw = password.to_string();
    let stored = hash.clone();
    let ok = super::blocking::run_kdf(move || verify_password(&pw, &stored)).await?;
    if !ok {
        return Err(Error::Unauthorized);
    }
    // Hashes from before the cost was raised are replaced at the next sign-in.
    if password_needs_rehash(&hash) {
        let pw = password.to_string();
        match super::blocking::run_kdf(move || hash_password(&pw)).await {
            Ok(new_hash) => {
                if let Err(e) = sqlx::query(
                    "UPDATE users SET password_hash = $2 WHERE id = $1 AND password_hash = $3",
                )
                .bind(user.id)
                .bind(&new_hash)
                .bind(&hash)
                .execute(pool)
                .await
                {
                    tracing::warn!(user_id = %user.id, error = %e, "password rehash failed");
                }
            }
            Err(e) => tracing::warn!(user_id = %user.id, error = %e, "password rehash failed"),
        }
    }
    Ok(user)
}
