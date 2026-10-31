use linkdean_core::crypto;
use linkdean_core::{Bookmark, BookmarkView, Error, Result, Vault};
use sqlx::PgPool;
use uuid::Uuid;

use super::bookmarks::bookmark_to_view;
use super::users::user_has_vault;

pub async fn search_vault_bookmarks(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
    vault_key: &[u8; 32],
) -> Result<Vec<BookmarkView>> {
    let rows = sqlx::query_as::<_, Bookmark>(
        "SELECT * FROM bookmarks WHERE owner_user_id = $1 AND is_vault = TRUE",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let q = query.to_ascii_lowercase();
    let mut out = Vec::new();
    for b in rows {
        if let Some(v) = bookmark_to_view(pool, b, Some(vault_key)).await? {
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

pub async fn setup_vault(pool: &PgPool, user_id: Uuid, passphrase: &str) -> Result<()> {
    if user_has_vault(pool, user_id).await? {
        return Err(Error::Conflict("vault already exists".into()));
    }
    let passphrase = passphrase.to_string();
    let keys = super::blocking::run_blocking(move || crypto::setup_vault(&passphrase)).await?;
    sqlx::query(
        r#"
        INSERT INTO vaults (user_id, kdf_salt, wrapped_key, wrapped_nonce)
        VALUES ($1, $2, $3, $4)
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
    sqlx::query_as::<_, Vault>("SELECT * FROM vaults WHERE user_id = $1")
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
