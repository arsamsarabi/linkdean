//! Organization SCIM tokens and membership provisioning (Phase D3).

use linkdean_core::crypto::{self, hash_token};
use linkdean_core::{
    CreateOrganizationScimToken, CreateOrganizationScimTokenResponse, Error, OrganizationRole,
    OrganizationScimToken, OrganizationScimTokenView, Result, User,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::email_verification::mark_email_verified;
use super::organization_sso::email_matches_org_verified_domain;
use super::organizations::{
    add_organization_member, membership_role, record_audit, require_org_role,
};
use super::users::{create_user_gated, find_user_by_email, find_user_by_id};

const TOKEN_PREFIX: &str = "ldscim_";

fn to_view(row: OrganizationScimToken) -> OrganizationScimTokenView {
    OrganizationScimTokenView {
        id: row.id,
        organization_id: row.organization_id,
        name: row.name,
        prefix: row.prefix,
        last_used_at: row.last_used_at,
        created_at: row.created_at,
    }
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

pub async fn list_organization_scim_tokens(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
) -> Result<Vec<OrganizationScimTokenView>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let rows = sqlx::query_as::<_, OrganizationScimToken>(
        r#"
        SELECT id, organization_id, name, token_hash, prefix, created_by_user_id,
               last_used_at, revoked_at, created_at, updated_at
        FROM organization_scim_tokens
        WHERE organization_id = $1 AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    Ok(rows.into_iter().map(to_view).collect())
}

/// Creates a token and revokes any previous active token for the org.
pub async fn create_organization_scim_token(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    req: CreateOrganizationScimToken,
) -> Result<CreateOrganizationScimTokenResponse> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
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
    let prefix = format!("{TOKEN_PREFIX}{}", &secret[..8.min(secret.len())]);

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE organization_scim_tokens
        SET revoked_at = now(), updated_at = now()
        WHERE organization_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(organization_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    let row = sqlx::query_as::<_, OrganizationScimToken>(
        r#"
        INSERT INTO organization_scim_tokens (
            organization_id, name, token_hash, prefix, created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, organization_id, name, token_hash, prefix, created_by_user_id,
                  last_used_at, revoked_at, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(&name)
    .bind(&token_hash)
    .bind(&prefix)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "scim.token_created",
        json!({ "token_id": row.id, "prefix": prefix }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;

    Ok(CreateOrganizationScimTokenResponse {
        token,
        view: to_view(row),
    })
}

pub async fn revoke_organization_scim_token(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    token_id: Uuid,
) -> Result<()> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let updated = sqlx::query(
        r#"
        UPDATE organization_scim_tokens
        SET revoked_at = now(), updated_at = now()
        WHERE id = $1 AND organization_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(token_id)
    .bind(organization_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    if updated.rows_affected() == 0 {
        return Err(Error::NotFound);
    }
    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "scim.token_revoked",
        json!({ "token_id": token_id }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// Resolves a live SCIM bearer token to its organization.
pub async fn authenticate_scim_token(pool: &PgPool, bearer: &str) -> Result<(Uuid, Uuid)> {
    let token = bearer.trim();
    if !token.starts_with(TOKEN_PREFIX) {
        return Err(Error::Unauthorized);
    }
    let hash = hash_token(token);
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT t.id, t.organization_id
        FROM organization_scim_tokens t
        JOIN organizations o ON o.id = t.organization_id AND o.deleted_at IS NULL
        WHERE t.token_hash = $1 AND t.revoked_at IS NULL
        "#,
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let Some((token_id, organization_id)) = row else {
        return Err(Error::Unauthorized);
    };
    let _ = sqlx::query(
        "UPDATE organization_scim_tokens SET last_used_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(token_id)
    .execute(pool)
    .await;
    Ok((token_id, organization_id))
}

async fn end_user_sessions(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

/// SCIM create/update: ensure user exists and is an org member (not guest).
pub async fn scim_provision_user(
    pool: &PgPool,
    organization_id: Uuid,
    email: &str,
    display_name: Option<&str>,
    active: bool,
) -> Result<User> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("userName/email required".into()));
    }
    if !email_matches_org_verified_domain(pool, organization_id, email).await? {
        return Err(Error::BadRequest(
            "email domain is not a verified domain of this organization".into(),
        ));
    }

    let user = match find_user_by_email(pool, email).await? {
        Some(u) => u,
        None => {
            let created = create_user_gated(
                pool,
                email,
                None,
                display_name,
                None,
                None,
                super::users::SignupGate {
                    enforce: false,
                    invite_code: None,
                    bootstrap_token_ok: None,
                    terms_version: None,
                    plan: None,
                },
            )
            .await?;
            let _ = mark_email_verified(pool, created.id).await;
            find_user_by_id(pool, created.id)
                .await?
                .ok_or(Error::NotFound)?
        }
    };

    if active {
        match add_organization_member(pool, organization_id, user.id, OrganizationRole::Member)
            .await
        {
            Ok(_) => {
                let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
                record_audit(
                    &mut tx,
                    organization_id,
                    None,
                    "scim.user_provisioned",
                    json!({ "user_id": user.id, "email": email }),
                )
                .await?;
                tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
            }
            Err(Error::Conflict(_)) => {
                let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
                record_audit(
                    &mut tx,
                    organization_id,
                    None,
                    "scim.user_updated",
                    json!({ "user_id": user.id, "active": true }),
                )
                .await?;
                tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
            }
            Err(e) => return Err(e),
        }
    } else {
        scim_deprovision_user(pool, organization_id, user.id).await?;
    }

    find_user_by_id(pool, user.id).await?.ok_or(Error::NotFound)
}

pub async fn scim_deprovision_user(
    pool: &PgPool,
    organization_id: Uuid,
    user_id: Uuid,
) -> Result<()> {
    let role = membership_role(pool, user_id, organization_id).await?;
    if role == Some(OrganizationRole::Owner) {
        let owners: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)::bigint FROM organization_members
            WHERE organization_id = $1 AND role = 'owner'
            "#,
        )
        .bind(organization_id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
        if owners <= 1 {
            return Err(Error::BadRequest(
                "cannot deprovision the last organization owner".into(),
            ));
        }
    }

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query(
        "DELETE FROM team_members WHERE user_id = $1 AND team_id IN (SELECT id FROM teams WHERE organization_id = $2)",
    )
    .bind(user_id)
    .bind(organization_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    sqlx::query("DELETE FROM organization_members WHERE organization_id = $1 AND user_id = $2")
        .bind(organization_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    record_audit(
        &mut tx,
        organization_id,
        None,
        "scim.user_deprovisioned",
        json!({ "user_id": user_id }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    end_user_sessions(pool, user_id).await?;
    Ok(())
}

pub async fn scim_list_org_members(
    pool: &PgPool,
    organization_id: Uuid,
    user_name_eq: Option<&str>,
) -> Result<Vec<User>> {
    if let Some(email) = user_name_eq {
        let Some(user) = find_user_by_email(pool, email).await? else {
            return Ok(vec![]);
        };
        if membership_role(pool, user.id, organization_id)
            .await?
            .is_some()
        {
            return Ok(vec![user]);
        }
        return Ok(vec![]);
    }
    let ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT user_id FROM organization_members
        WHERE organization_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(u) = find_user_by_id(pool, id).await? {
            out.push(u);
        }
    }
    Ok(out)
}

pub async fn scim_get_org_member(
    pool: &PgPool,
    organization_id: Uuid,
    user_id: Uuid,
) -> Result<User> {
    membership_role(pool, user_id, organization_id)
        .await?
        .ok_or(Error::NotFound)?;
    find_user_by_id(pool, user_id).await?.ok_or(Error::NotFound)
}

pub fn is_scim_token(token: &str) -> bool {
    token.starts_with(TOKEN_PREFIX)
}

#[allow(dead_code)]
pub fn scim_email_domain(email: &str) -> Option<String> {
    email_domain(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scim_token_prefix_and_domain() {
        assert!(is_scim_token("ldscim_abc"));
        assert!(!is_scim_token("ldapi_abc"));
        assert_eq!(
            email_domain("a@Acme.Example.com."),
            Some("acme.example.com".into())
        );
        assert_eq!(email_domain("nodomain"), None);
    }
}
