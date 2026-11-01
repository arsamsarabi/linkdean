use linkdean_core::crypto;
use linkdean_core::{Error, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::vault::{get_user_vault, list_vaults};

#[derive(sqlx::FromRow)]
struct MasterKeyRow {
    kdf_salt: Vec<u8>,
    wrapped_key: Vec<u8>,
    wrapped_nonce: Vec<u8>,
}

async fn master_row(pool: &PgPool, user_id: Uuid) -> Result<Option<MasterKeyRow>> {
    sqlx::query_as::<_, MasterKeyRow>(
        "SELECT kdf_salt, wrapped_key, wrapped_nonce FROM vault_master_keys WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

async fn open_master(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<[u8; 32]> {
    let row = master_row(pool, user_id).await?.ok_or(Error::NotFound)?;
    let passphrase = passphrase.to_string();
    super::blocking::run_blocking(move || {
        crypto::unlock_vault(
            &passphrase,
            &row.kdf_salt,
            &row.wrapped_key,
            &row.wrapped_nonce,
        )
    })
    .await
}

pub async fn has_global_passphrase(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    Ok(master_row(pool, user_id).await?.is_some())
}

pub async fn enable_global_passphrase(
    pool: &PgPool,
    user_id: Uuid,
    passphrase: &str,
) -> Result<()> {
    if has_global_passphrase(pool, user_id).await? {
        return Err(Error::Conflict("Global passphrase is already on.".into()));
    }
    let passphrase = passphrase.to_string();
    let wrapped = super::blocking::run_blocking(move || {
        crypto::wrap_data_key(&passphrase, &crypto::new_data_key())
    })
    .await?;
    sqlx::query(
        r#"
        INSERT INTO vault_master_keys (user_id, kdf_salt, wrapped_key, wrapped_nonce)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(&wrapped.kdf_salt)
    .bind(&wrapped.wrapped_key)
    .bind(&wrapped.wrapped_nonce)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Only the master key is rewrapped, so joined vaults stay joined.
pub async fn change_global_passphrase(
    pool: &PgPool,
    user_id: Uuid,
    current: &str,
    new_passphrase: &str,
) -> Result<()> {
    let master = open_master(pool, user_id, current).await?;
    let new_passphrase = new_passphrase.to_string();
    let wrapped =
        super::blocking::run_blocking(move || crypto::wrap_data_key(&new_passphrase, &master))
            .await?;
    sqlx::query(
        r#"
        UPDATE vault_master_keys
        SET kdf_salt = $1, wrapped_key = $2, wrapped_nonce = $3, updated_at = now()
        WHERE user_id = $4
        "#,
    )
    .bind(&wrapped.kdf_salt)
    .bind(&wrapped.wrapped_key)
    .bind(&wrapped.wrapped_nonce)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Removes the global path only. No vault contents or passphrases change.
pub async fn disable_global_passphrase(pool: &PgPool, user_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET global_wrapped_key = NULL, global_wrapped_nonce = NULL, updated_at = now()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query("DELETE FROM vault_master_keys WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))
}

/// The caller holds the vault's unlocked data key.
pub async fn join_global(
    pool: &PgPool,
    user_id: Uuid,
    vault_id: Uuid,
    data_key: &[u8; 32],
    passphrase: &str,
) -> Result<()> {
    get_user_vault(pool, user_id, vault_id).await?;
    let master = open_master(pool, user_id, passphrase).await?;
    let (ct, nonce) = crypto::wrap_with_key(&master, data_key)?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET global_wrapped_key = $1, global_wrapped_nonce = $2, updated_at = now()
        WHERE id = $3
        "#,
    )
    .bind(ct)
    .bind(nonce)
    .bind(vault_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn leave_global(pool: &PgPool, user_id: Uuid, vault_id: Uuid) -> Result<()> {
    get_user_vault(pool, user_id, vault_id).await?;
    sqlx::query(
        r#"
        UPDATE vaults
        SET global_wrapped_key = NULL, global_wrapped_nonce = NULL, updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(vault_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Keys of every joined vault.
pub async fn unlock_with_global(
    pool: &PgPool,
    user_id: Uuid,
    passphrase: &str,
) -> Result<Vec<(Uuid, [u8; 32])>> {
    let master = open_master(pool, user_id, passphrase).await?;
    let mut out = Vec::new();
    for vault in list_vaults(pool, user_id).await? {
        if let (Some(ct), Some(nonce)) = (vault.global_wrapped_key, vault.global_wrapped_nonce) {
            out.push((vault.id, crypto::unwrap_with_key(&master, &ct, &nonce)?));
        }
    }
    Ok(out)
}
