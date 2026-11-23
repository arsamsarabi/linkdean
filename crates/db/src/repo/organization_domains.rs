//! Organization verified domains (Phase C).

use linkdean_core::crypto::random_bytes;
use linkdean_core::{
    CreateOrganizationDomain, DomainJoinMode, Error, OrganizationDomain, OrganizationDomainView,
    OrganizationRole, PatchOrganizationDomain, Result,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use super::organizations::{record_audit, require_org_role};

fn parse_join_mode(s: &str) -> Result<DomainJoinMode> {
    DomainJoinMode::parse(s).map_err(|_| Error::msg(format!("invalid join_mode `{s}`")))
}

fn challenge_name(domain: &str) -> String {
    format!("_linkdean-challenge.{}", domain.trim_end_matches('.'))
}

fn challenge_value(token: &str) -> String {
    format!("linkdean-domain-verification={token}")
}

fn to_view(row: OrganizationDomain) -> Result<OrganizationDomainView> {
    Ok(OrganizationDomainView {
        id: row.id,
        organization_id: row.organization_id,
        txt_name: challenge_name(&row.domain),
        txt_value: challenge_value(&row.verification_token),
        domain: row.domain,
        verified_at: row.verified_at,
        join_mode: parse_join_mode(&row.join_mode)?,
        created_at: row.created_at,
    })
}

/// Normalize and validate an apex/host domain for verification.
pub fn normalize_domain(raw: &str) -> Result<String> {
    let d = raw.trim().trim_end_matches('.').to_ascii_lowercase();
    if d.is_empty() {
        return Err(Error::BadRequest("domain required".into()));
    }
    if d.contains('@') || d.contains('/') || d.contains(' ') {
        return Err(Error::BadRequest("invalid domain".into()));
    }
    if !d.contains('.') {
        return Err(Error::BadRequest(
            "use a full domain (e.g. acme.com)".into(),
        ));
    }
    if d.len() > 253 {
        return Err(Error::BadRequest("domain too long".into()));
    }
    if !d
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return Err(Error::BadRequest("invalid domain".into()));
    }
    Ok(d)
}

pub fn refuse_blocked_domain(domain: &str) -> Result<()> {
    // Disposable list lives in the API crate; DB layer rejects the same consumer set
    // and a small disposable set so add() cannot be bypassed via repo tests alone.
    const BLOCKED: &[&str] = &[
        "gmail.com",
        "googlemail.com",
        "outlook.com",
        "hotmail.com",
        "live.com",
        "msn.com",
        "yahoo.com",
        "ymail.com",
        "icloud.com",
        "me.com",
        "mac.com",
        "aol.com",
        "proton.me",
        "protonmail.com",
        "mailinator.com",
        "guerrillamail.com",
        "yopmail.com",
        "tempmail.com",
        "10minutemail.com",
        "trashmail.com",
    ];
    let d = domain.to_ascii_lowercase();
    if BLOCKED
        .iter()
        .any(|b| d == *b || d.ends_with(&format!(".{b}")))
    {
        return Err(Error::BadRequest(
            "That domain can't be verified for an organization.".into(),
        ));
    }
    Ok(())
}

pub async fn list_organization_domains(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
) -> Result<Vec<OrganizationDomainView>> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let rows = sqlx::query_as::<_, OrganizationDomain>(
        r#"
        SELECT id, organization_id, domain, verification_token, verified_at, join_mode,
               created_by_user_id, created_at, updated_at
        FROM organization_domains
        WHERE organization_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    rows.into_iter().map(to_view).collect()
}

pub async fn create_organization_domain(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    req: CreateOrganizationDomain,
) -> Result<OrganizationDomainView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let domain = normalize_domain(&req.domain)?;
    refuse_blocked_domain(&domain)?;
    let token = hex::encode(random_bytes(32));

    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let row = sqlx::query_as::<_, OrganizationDomain>(
        r#"
        INSERT INTO organization_domains (
            organization_id, domain, verification_token, created_by_user_id
        )
        VALUES ($1, $2, $3, $4)
        RETURNING id, organization_id, domain, verification_token, verified_at, join_mode,
                  created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(&domain)
    .bind(&token)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if e.as_database_error()
            .and_then(|d| d.constraint())
            .is_some_and(|c| c.contains("domain"))
            || e.to_string().contains("unique")
            || e.to_string().contains("duplicate")
        {
            Error::Conflict("That domain is already claimed.".into())
        } else {
            Error::msg(e.to_string())
        }
    })?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "domain_added",
        json!({ "domain_id": row.id, "domain": domain }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    to_view(row)
}

pub async fn delete_organization_domain(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    domain_id: Uuid,
) -> Result<()> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let domain: Option<String> = sqlx::query_scalar(
        r#"
        DELETE FROM organization_domains
        WHERE id = $1 AND organization_id = $2
        RETURNING domain
        "#,
    )
    .bind(domain_id)
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let Some(domain) = domain else {
        return Err(Error::NotFound);
    };
    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "domain_removed",
        json!({ "domain_id": domain_id, "domain": domain }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    Ok(())
}

pub async fn patch_organization_domain(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    domain_id: Uuid,
    req: PatchOrganizationDomain,
) -> Result<OrganizationDomainView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let row = sqlx::query_as::<_, OrganizationDomain>(
        r#"
        UPDATE organization_domains
        SET join_mode = $3, updated_at = now()
        WHERE id = $1 AND organization_id = $2
        RETURNING id, organization_id, domain, verification_token, verified_at, join_mode,
                  created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(domain_id)
    .bind(organization_id)
    .bind(req.join_mode.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "domain_join_mode_changed",
        json!({
            "domain_id": domain_id,
            "domain": row.domain,
            "join_mode": req.join_mode.as_str(),
        }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    to_view(row)
}

/// Mark verified when caller already confirmed TXT records contain the token.
pub async fn mark_organization_domain_verified(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    domain_id: Uuid,
) -> Result<OrganizationDomainView> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    let mut tx = pool.begin().await.map_err(|e| Error::msg(e.to_string()))?;
    let existing = sqlx::query_as::<_, OrganizationDomain>(
        r#"
        SELECT id, organization_id, domain, verification_token, verified_at, join_mode,
               created_by_user_id, created_at, updated_at
        FROM organization_domains
        WHERE id = $1 AND organization_id = $2
        "#,
    )
    .bind(domain_id)
    .bind(organization_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)?;

    if existing.verified_at.is_some() {
        return to_view(existing);
    }

    let row = sqlx::query_as::<_, OrganizationDomain>(
        r#"
        UPDATE organization_domains
        SET verified_at = now(), updated_at = now()
        WHERE id = $1 AND organization_id = $2 AND verified_at IS NULL
        RETURNING id, organization_id, domain, verification_token, verified_at, join_mode,
                  created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(domain_id)
    .bind(organization_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;

    record_audit(
        &mut tx,
        organization_id,
        Some(actor_id),
        "domain_verified",
        json!({ "domain_id": domain_id, "domain": row.domain }),
    )
    .await?;
    tx.commit().await.map_err(|e| Error::msg(e.to_string()))?;
    to_view(row)
}

/// Load a domain row for an admin (includes token for TXT verify).
pub async fn get_organization_domain_for_admin(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    domain_id: Uuid,
) -> Result<OrganizationDomain> {
    require_org_role(pool, actor_id, organization_id, OrganizationRole::Admin).await?;
    sqlx::query_as::<_, OrganizationDomain>(
        r#"
        SELECT id, organization_id, domain, verification_token, verified_at, join_mode,
               created_by_user_id, created_at, updated_at
        FROM organization_domains
        WHERE id = $1 AND organization_id = $2
        "#,
    )
    .bind(domain_id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?
    .ok_or(Error::NotFound)
}

/// Confirm TXT strings match the stored token, then mark verified.
pub async fn verify_organization_domain_with_txt(
    pool: &PgPool,
    actor_id: Uuid,
    organization_id: Uuid,
    domain_id: Uuid,
    txt_records: &[String],
) -> Result<OrganizationDomainView> {
    let row = get_organization_domain_for_admin(pool, actor_id, organization_id, domain_id).await?;
    if row.verified_at.is_some() {
        return to_view(row);
    }
    let want = challenge_value(&row.verification_token);
    let ok = txt_records.iter().any(|r| {
        r.split(|c: char| c.is_whitespace() || c == '"')
            .any(|part| part == want)
            || r.contains(&want)
    });
    if !ok {
        return Err(Error::BadRequest(
            "TXT record not found. Publish the challenge value, wait for DNS, then try again."
                .into(),
        ));
    }
    mark_organization_domain_verified(pool, actor_id, organization_id, domain_id).await
}
