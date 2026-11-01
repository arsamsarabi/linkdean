use linkdean_core::crypto;
use linkdean_core::{Bookmark, BookmarkView, Error, Result, User, Vault, VaultKeyring};
use sqlx::PgPool;
use uuid::Uuid;

use super::bookmarks::bookmark_to_view;
use super::users::user_has_vault;

pub async fn search_vault_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
    keyring: &VaultKeyring,
) -> Result<Vec<BookmarkView>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        "SELECT * FROM bookmarks WHERE owner_user_id = $1 AND vault_id = ANY($2)",
    )
    .bind(user_id)
    .bind(keyring.vault_ids())
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let q = query.to_ascii_lowercase();
    let mut out = Vec::new();
    for b in rows {
        if let Some(v) = bookmark_to_view(pool, b, keyring, user_id).await? {
            let hay = format!(
                "{} {} {} {} {}",
                v.title,
                v.url,
                v.description.clone().unwrap_or_default(),
                v.notes.clone().unwrap_or_default(),
                v.tags.join(" ")
            )
            .to_ascii_lowercase();
            if hay.contains(&q) {
                out.push(v);
            }
        }
    }
    Ok(out)
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(Error::BadRequest(
            "vault name must be 1-60 characters".into(),
        ));
    }
    Ok(name.to_string())
}

fn map_name_conflict(e: sqlx::Error) -> Error {
    match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("vaults_user_name_uidx") => {
            Error::Conflict("You already have a vault with this name.".into())
        }
        _ => Error::msg(e.to_string()),
    }
}

/// Create a vault. Returns the row, its data key and its recovery key, shown once.
pub async fn create_vault(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    passphrase: &str,
) -> Result<(Vault, [u8; 32], String)> {
    let name = clean_name(name)?;
    let passphrase = passphrase.to_string();
    let (key, by_passphrase, recovery, by_recovery) = super::blocking::run_blocking(move || {
        let key = crypto::new_data_key();
        let by_passphrase = crypto::wrap_data_key(&passphrase, &key)?;
        let recovery = crypto::new_recovery_key();
        let by_recovery = crypto::wrap_data_key(&crypto::normalize_recovery_key(&recovery), &key)?;
        Ok((key, by_passphrase, recovery, by_recovery))
    })
    .await?;

    let vault = sqlx::query_as::<_, Vault>(
        r#"
        INSERT INTO vaults (
            user_id, name, kdf_salt, wrapped_key, wrapped_nonce,
            recovery_salt, recovery_wrapped_key, recovery_wrapped_nonce
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(&by_passphrase.kdf_salt)
    .bind(&by_passphrase.wrapped_key)
    .bind(&by_passphrase.wrapped_nonce)
    .bind(&by_recovery.kdf_salt)
    .bind(&by_recovery.wrapped_key)
    .bind(&by_recovery.wrapped_nonce)
    .fetch_one(pool)
    .await
    .map_err(map_name_conflict)?;
    Ok((vault, key, recovery))
}

pub async fn list_vaults(pool: &PgPool, user_id: Uuid) -> Result<Vec<Vault>> {
    sqlx::query_as::<_, Vault>("SELECT * FROM vaults WHERE user_id = $1 ORDER BY lower(name)")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_user_vault(pool: &PgPool, user_id: Uuid, id: Uuid) -> Result<Vault> {
    sqlx::query_as::<_, Vault>("SELECT * FROM vaults WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .ok_or(Error::NotFound)
}

/// Active categories and bookmarks in a vault.
pub async fn vault_item_counts(pool: &PgPool, vault_id: Uuid) -> Result<(i64, i64)> {
    sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM categories WHERE vault_id = $1 AND deleted_at IS NULL),
            (SELECT COUNT(*) FROM bookmarks WHERE vault_id = $1 AND deleted_at IS NULL)
        "#,
    )
    .bind(vault_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn rename_vault(pool: &PgPool, user_id: Uuid, id: Uuid, name: &str) -> Result<Vault> {
    let name = clean_name(name)?;
    get_user_vault(pool, user_id, id).await?;
    sqlx::query_as::<_, Vault>(
        "UPDATE vaults SET name = $1, updated_at = now() WHERE id = $2 RETURNING *",
    )
    .bind(name)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(map_name_conflict)
}

pub async fn unlock_vault(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    passphrase: &str,
) -> Result<[u8; 32]> {
    let vault = get_user_vault(pool, user_id, id).await?;
    let passphrase = passphrase.to_string();
    super::blocking::run_blocking(move || {
        crypto::unlock_vault(
            &passphrase,
            &vault.kdf_salt,
            &vault.wrapped_key,
            &vault.wrapped_nonce,
        )
    })
    .await
}

/// Set a new passphrase using the recovery key. Returns the data key.
pub async fn recover_vault(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    recovery_key: &str,
    new_passphrase: &str,
) -> Result<[u8; 32]> {
    let vault = get_user_vault(pool, user_id, id).await?;
    let (Some(salt), Some(wrapped), Some(nonce)) = (
        vault.recovery_salt,
        vault.recovery_wrapped_key,
        vault.recovery_wrapped_nonce,
    ) else {
        return Err(Error::VaultUnlockFailed);
    };
    let recovery = crypto::normalize_recovery_key(recovery_key);
    let new_passphrase = new_passphrase.to_string();
    let (key, by_passphrase) = super::blocking::run_blocking(move || {
        let key = crypto::unlock_vault(&recovery, &salt, &wrapped, &nonce)?;
        let by_passphrase = crypto::wrap_data_key(&new_passphrase, &key)?;
        Ok((key, by_passphrase))
    })
    .await?;

    sqlx::query(
        r#"
        UPDATE vaults
        SET kdf_salt = $1, wrapped_key = $2, wrapped_nonce = $3, updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(&by_passphrase.kdf_salt)
    .bind(&by_passphrase.wrapped_key)
    .bind(&by_passphrase.wrapped_nonce)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(key)
}

/// Create or replace the recovery key. The caller holds the unlocked data key.
pub async fn replace_recovery_key(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    data_key: &[u8; 32],
) -> Result<String> {
    get_user_vault(pool, user_id, id).await?;
    let key = *data_key;
    let (recovery, wrapped) = super::blocking::run_blocking(move || {
        let recovery = crypto::new_recovery_key();
        let wrapped = crypto::wrap_data_key(&crypto::normalize_recovery_key(&recovery), &key)?;
        Ok((recovery, wrapped))
    })
    .await?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET recovery_salt = $1, recovery_wrapped_key = $2, recovery_wrapped_nonce = $3,
            updated_at = now()
        WHERE id = $4
        "#,
    )
    .bind(&wrapped.kdf_salt)
    .bind(&wrapped.wrapped_key)
    .bind(&wrapped.wrapped_nonce)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(recovery)
}

/// Permanently delete a vault with its categories and bookmarks.
pub async fn delete_vault(
    pool: &PgPool,
    user: &User,
    id: Uuid,
    typed_name: &str,
    password: Option<&str>,
) -> Result<()> {
    let vault = get_user_vault(pool, user.id, id).await?;
    if typed_name != vault.name {
        return Err(Error::BadRequest("The vault name doesn't match.".into()));
    }
    if let Some(hash) = user.password_hash.clone() {
        let password = password.unwrap_or_default().to_string();
        let ok = super::blocking::run_blocking(move || crypto::verify_password(&password, &hash))
            .await?;
        if !ok {
            return Err(Error::BadRequest(
                "The account password is incorrect.".into(),
            ));
        }
    }
    sqlx::query("DELETE FROM vaults WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn setup_vault(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<()> {
    if user_has_vault(pool, user_id).await? {
        return Err(Error::Conflict("vault already exists".into()));
    }
    let passphrase = passphrase.to_string();
    let keys = super::blocking::run_blocking(move || crypto::setup_vault(&passphrase)).await?;
    sqlx::query(
        r#"
        INSERT INTO vaults (user_id, name, kdf_salt, wrapped_key, wrapped_nonce)
        VALUES ($1, 'Vault', $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(&keys.kdf_salt)
    .bind(&keys.wrapped_key)
    .bind(&keys.wrapped_nonce)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn get_vault(pool: &PgPool, user_id: Uuid) -> Result<Vault> {
    sqlx::query_as::<_, Vault>(
        "SELECT * FROM vaults WHERE user_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

pub async fn unlock_vault_key(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<[u8; 32]> {
    let vault = get_vault(pool, user_id).await?;
    let passphrase = passphrase.to_string();
    let salt = vault.kdf_salt.clone();
    let wrapped_key = vault.wrapped_key.clone();
    let wrapped_nonce = vault.wrapped_nonce.clone();
    super::blocking::run_blocking(move || {
        crypto::unlock_vault(&passphrase, &salt, &wrapped_key, &wrapped_nonce)
    })
    .await
}
