//! Per-organization OIDC SSO config (Phase D1).

use linkdean_core::{
    DiscoverOrgSsoResponse, Error, OrganizationRole, OrganizationSsoConfig,
    OrganizationSsoConfigView, Result,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::organization_domains::normalize_domain;
use super::organizations::{record_audit, require_org_role};

fn to_view(row: OrganizationSsoConfig, redirect_uri: &str) -> OrganizationSsoConfigView {
    OrganizationSsoConfigView {
        organization_id: row.organization_id,
        issuer_url: row.issuer_url,
        client_id: row.client_id,
        display_name: row.display_name,
        enabled: row.enabled,
        redirect_uri: redirect_uri.to_string(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn count_verified_domains(pool: &PgPool, organization_id: Uuid) -> Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint FROM organization_domains
        WHERE organization_id = $1 AND verified_at IS NOT NULL
        "#,
    )
    .bind(organization_id)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

fn email_domain(email: &str) -> Option<String> {
    let email = email.trim();
    let at = email.rfind('@')?;
    let domain = email[at + 1..]
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

/// Load SSO config for an org admin (ciphertext included; never return ciphertext to clients).
pub async fn get_organization_sso_row(
    pool: &PgPool,
    organization_id: Uuid,
) -> Result<Option<OrganizationSsoConfig>> {
    sqlx::query_as::<_, OrganizationSsoConfig>(
        r#"
        SELECT organization_id, issuer_url, client_id, client_secret_ciphertext, client_secret_nonce,
               display_name, enabled, created_by_user_id, created_at, updated_at
        FROM organization_sso_configs
        WHERE organization_id = $1
        "#,
    )
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))
}

pub async fn get_organization_sso(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    redirect_uri: &str,
) -> Result<Option<OrganizationSsoConfigView>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    Ok(get_organization_sso_row(pool, organization_id)
        .await?
        .map(|row| to_view(row, redirect_uri)))
}

/// Upsert SSO config. `secret` is ciphertext+nonce when the client secret changes; `None` keeps
/// the existing secret (required when creating).
pub async fn upsert_organization_sso(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    issuer_url: &str,
    client_id: &str,
    secret: Option<(Vec<u8>, Vec<u8>)>,
    display_name: &str,
    enabled: bool,
    redirect_uri: &str,
) -> Result<OrganizationSsoConfigView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;

    let issuer_url = issuer_url.trim();
    let client_id = client_id.trim();
    let display_name = {
        let d = display_name.trim();
        if d.is_empty() {
            "Company SSO"
        } else {
            d
        }
    };
    if issuer_url.is_empty() {
        return Err(Error::BadRequest("issuer_url required".into()));
    }
    if client_id.is_empty() {
        return Err(Error::BadRequest("client_id required".into()));
    }
    if !(issuer_url.starts_with("https://") || issuer_url.starts_with("http://")) {
        return Err(Error::BadRequest(
            "issuer_url must be an http(s) URL".into(),
        ));
    }
    if enabled && count_verified_domains(pool, organization_id).await? < 1 {
        return Err(Error::BadRequest(
            "Verify at least one domain before enabling organization SSO.".into(),
        ));
    }

    let existing = get_organization_sso_row(pool, organization_id).await?;
    let (ciphertext, nonce) = match (&existing, &secret) {
        (_, Some((ct, n))) => (ct.clone(), n.clone()),
        (Some(row), None) => (
            row.client_secret_ciphertext.clone(),
            row.client_secret_nonce.clone(),
        ),
        (None, None) => {
            return Err(Error::BadRequest(
                "client_secret required when configuring SSO".into(),
            ));
        }
    };

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let row = sqlx::query_as::<_, OrganizationSsoConfig>(
        r#"
        INSERT INTO organization_sso_configs (
            organization_id, issuer_url, client_id, client_secret_ciphertext, client_secret_nonce,
            display_name, enabled, created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (organization_id) DO UPDATE SET
            issuer_url = EXCLUDED.issuer_url,
            client_id = EXCLUDED.client_id,
            client_secret_ciphertext = EXCLUDED.client_secret_ciphertext,
            client_secret_nonce = EXCLUDED.client_secret_nonce,
            display_name = EXCLUDED.display_name,
            enabled = EXCLUDED.enabled,
            updated_at = now()
        RETURNING organization_id, issuer_url, client_id, client_secret_ciphertext, client_secret_nonce,
                  display_name, enabled, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(issuer_url)
    .bind(client_id)
    .bind(&ciphertext)
    .bind(&nonce)
    .bind(display_name)
    .bind(enabled)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let was_enabled = existing.as_ref().map(|e| e.enabled).unwrap_or(false);
    if existing.is_none() {
        record_audit(
            &mut tx,
            organization_id,
            Some(actor_id),
            "sso.configured",
            json!({ "issuer_url": issuer_url, "enabled": enabled }),
        )
        .await?;
    }
    if enabled && !was_enabled {
        record_audit(
            &mut tx,
            organization_id,
            Some(actor_id),
            "sso.enabled",
            json!({ "issuer_url": issuer_url }),
        )
        .await?;
    } else if !enabled && was_enabled {
        record_audit(
            &mut tx,
            organization_id,
            Some(actor_id),
            "sso.disabled",
            json!({}),
        )
        .await?;
    }

    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(to_view(row, redirect_uri))
}

pub async fn delete_organization_sso(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
) -> Result<()> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let deleted = sqlx::query(
        "DELETE FROM organization_sso_configs WHERE organization_id = $1 RETURNING organization_id",
    )
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if deleted.is_none() {
        return Err(Error::NotFound);
    }
    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "sso.disabled",
        json!({ "deleted": true }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Public discover: email domain → enabled org SSO, or a non-leaking `{ sso: false }`.
pub async fn discover_organization_sso(
    pool: &PgPool,
    email: &str,
) -> Result<DiscoverOrgSsoResponse> {
    let Some(domain) = email_domain(email) else {
        return Ok(DiscoverOrgSsoResponse {
            sso: false,
            organization_id: None,
            organization_name: None,
        });
    };
    let Ok(domain) = normalize_domain(&domain) else {
        return Ok(DiscoverOrgSsoResponse {
            sso: false,
            organization_id: None,
            organization_name: None,
        });
    };

    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT o.id, o.name
        FROM organization_domains d
        JOIN organizations o ON o.id = d.organization_id AND o.deleted_at IS NULL
        JOIN organization_sso_configs s ON s.organization_id = o.id AND s.enabled
        WHERE d.verified_at IS NOT NULL AND d.domain = $1
        LIMIT 1
        "#,
    )
    .bind(&domain)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    Ok(match row {
        Some((organization_id, organization_name)) => DiscoverOrgSsoResponse {
            sso: true,
            organization_id: Some(organization_id),
            organization_name: Some(organization_name),
        },
        None => DiscoverOrgSsoResponse {
            sso: false,
            organization_id: None,
            organization_name: None,
        },
    })
}

/// Enabled SSO row for org-oidc start/callback (no admin check).
pub async fn get_enabled_organization_sso(
    pool: &PgPool,
    organization_id: Uuid,
) -> Result<OrganizationSsoConfig> {
    let row = get_organization_sso_row(pool, organization_id)
        .await?
        .ok_or_else(|| Error::BadRequest("organization SSO is not configured".into()))?;
    if !row.enabled {
        return Err(Error::BadRequest("organization SSO is not enabled".into()));
    }
    if count_verified_domains(pool, organization_id).await? < 1 {
        return Err(Error::BadRequest(
            "organization SSO requires a verified domain".into(),
        ));
    }
    Ok(row)
}

/// Whether `email`'s domain is a verified domain of `organization_id`.
pub async fn email_matches_org_verified_domain(
    pool: &PgPool,
    organization_id: Uuid,
    email: &str,
) -> Result<bool> {
    let Some(domain) = email_domain(email) else {
        return Ok(false);
    };
    let Ok(domain) = normalize_domain(&domain) else {
        return Ok(false);
    };
    let found: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id FROM organization_domains
        WHERE organization_id = $1 AND verified_at IS NOT NULL AND domain = $2
        LIMIT 1
        "#,
    )
    .bind(organization_id)
    .bind(&domain)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(found.is_some())
}

pub async fn record_sso_login(pool: &PgPool, organization_id: Uuid, user_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    record_audit(
        &mut tx,
        organization_id,
        Some(user_id),
        "sso.login",
        json!({ "user_id": user_id }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}
