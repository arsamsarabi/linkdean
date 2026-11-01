use chrono::Utc;
use linkdean_core::crypto::{self, hash_token};
use linkdean_core::{
    ApiKey, ApiKeyView, CreateApiKey, CreateApiKeyResponse, Error, Result, User,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::platform::get_platform_settings;
use super::users::find_user_by_id;

const TOKEN_PREFIX: &str = "ldk_";

fn to_view(row: ApiKey) -> ApiKeyView {
    ApiKeyView {
        id: row.id,
        name: row.name,
        prefix: row.prefix,
        last_used_at: row.last_used_at,
        revoked_at: row.revoked_at,
        created_at: row.created_at,
    }
}

/// Platform + per-user gates must both be on.
pub async fn require_api_keys_allowed(pool: &PgPool, user: &User) -> Result<()> {
    let platform = get_platform_settings(pool).await?;
    if !platform.api_keys_enabled {
        return Err(Error::Forbidden);
    }
    if !user.api_keys_enabled {
        return Err(Error::Forbidden);
    }
    Ok(())
}

pub async fn list_api_keys(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiKeyView>> {
    let rows = sqlx::query_as::<_, ApiKey>(
        r#"
        SELECT *
        FROM api_keys
        WHERE user_id = $1 AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(to_view).collect())
}

pub async fn create_api_key(
    pool: &PgPool,
    user: &User,
    req: CreateApiKey,
) -> Result<CreateApiKeyResponse> {
    require_api_keys_allowed(pool, user).await?;
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(Error::BadRequest("name required".into()));
    }
    if name.len() > 120 {
        return Err(Error::BadRequest("name max 120 chars".into()));
    }

    let secret = hex::encode(crypto::random_bytes(32));
    let token = format!("{TOKEN_PREFIX}{secret}");
    let token_hash = hash_token(&token);
    // Showable prefix: `ldk_` + first 8 hex of secret
    let prefix = format!("{TOKEN_PREFIX}{}", &secret[..8.min(secret.len())]);

    let row = sqlx::query_as::<_, ApiKey>(
        r#"
        INSERT INTO api_keys (user_id, name, token_hash, prefix)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user.id)
    .bind(&name)
    .bind(&token_hash)
    .bind(&prefix)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok(CreateApiKeyResponse {
        key: to_view(row),
        token,
    })
}

pub async fn revoke_api_key(pool: &PgPool, user_id: Uuid, key_id: Uuid) -> Result<()> {
    let res = sqlx::query(
        r#"
        UPDATE api_keys
        SET revoked_at = now()
        WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(key_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if res.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub async fn user_from_api_key(pool: &PgPool, token: &str) -> Result<Option<User>> {
    if !token.starts_with(TOKEN_PREFIX) {
        return Ok(None);
    }
    let token_hash = hash_token(token);
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT user_id FROM api_keys
        WHERE token_hash = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let Some((uid,)) = row else {
        return Ok(None);
    };
    let Some(user) = find_user_by_id(pool, uid).await? else {
        return Ok(None);
    };

    // Soft-fail gates: disabled → treat as unauthorized (no user)
    if require_api_keys_allowed(pool, &user).await.is_err() {
        return Ok(None);
    }

    let _ = sqlx::query(
        r#"
        UPDATE api_keys SET last_used_at = $1
        WHERE token_hash = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(Utc::now())
    .bind(&token_hash)
    .execute(pool)
    .await;

    Ok(Some(user))
}
