//! Managed Pro checkout via Stripe (Plans Phase C).
//!
//! Session-only Checkout and Customer Portal. Webhooks update `users.plan` through
//! [`db::set_user_plan`]. Self-host and managed without Stripe keys leave these routes off.

use crate::auth::SessionUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::stripe_client;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use linkdean_core::{
    plan_change_from_event, plan_str, verify_webhook_signature, Error, PlanChange, User,
};
use linkdean_db as db;
use serde::Serialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(checkout))
        .routes(routes!(portal))
}

/// Stripe webhook (raw body). Merged outside OpenAPI.
pub fn webhook_router() -> Router<AppState> {
    Router::new().route("/billing/webhook", post(webhook))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
struct BillingUrl {
    url: String,
}

fn billing_or_not_found(state: &AppState) -> Result<&crate::config::StripeBilling, Error> {
    if !state.cfg.billing_enabled() {
        return Err(Error::NotFound);
    }
    state.cfg.stripe.as_ref().ok_or(Error::NotFound)
}

/// Creates a Stripe Checkout Session for personal Pro and returns the hosted URL.
#[utoipa::path(
    post,
    operation_id = "billing_checkout",
    path = "/billing/checkout",
    tag = "billing",
    security(("session_token" = []), ("session_cookie" = [])),
    responses(
        (status = 200, description = "OK", body = BillingUrl),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn checkout(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
) -> ApiResult<Json<BillingUrl>> {
    let billing = billing_or_not_found(&state)?;
    if user.plan.as_deref() == Some("pro") {
        return Err(Error::BadRequest(
            "already on Pro; use Manage billing to change or cancel".into(),
        )
        .into());
    }
    let customer_id = ensure_customer(&state, billing, &user).await?;
    let url =
        stripe_client::create_checkout_session(billing, &state.cfg.app_url, user.id, &customer_id)
            .await?;
    Ok(Json(BillingUrl { url }))
}

/// Creates a Stripe Customer Portal session for the signed-in account.
#[utoipa::path(
    post,
    operation_id = "billing_portal",
    path = "/billing/portal",
    tag = "billing",
    security(("session_token" = []), ("session_cookie" = [])),
    responses(
        (status = 200, description = "OK", body = BillingUrl),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn portal(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
) -> ApiResult<Json<BillingUrl>> {
    let billing = billing_or_not_found(&state)?;
    let customer_id = user
        .stripe_customer_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            Error::BadRequest("no billing customer yet; start with Upgrade to Pro".into())
        })?;
    let url =
        stripe_client::create_portal_session(billing, &state.cfg.app_url, customer_id).await?;
    Ok(Json(BillingUrl { url }))
}

/// Stripe webhook. Verifies `Stripe-Signature` and applies plan changes.
async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let billing = billing_or_not_found(&state)?;
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let now = chrono::Utc::now().timestamp();
    if !verify_webhook_signature(&billing.webhook_secret, &body, sig, now) {
        return Err(Error::Unauthorized.into());
    }
    // Authenticated by Stripe-Signature above. Cap size, require UTF-8, then parse as a plain
    // `Value` (no #[typetag] / trait objects) so the only risk is a large JSON structure.
    const MAX_WEBHOOK_BYTES: usize = 1_048_576;
    if body.len() > MAX_WEBHOOK_BYTES {
        return Err(Error::BadRequest("webhook body too large".into()).into());
    }
    let text = std::str::from_utf8(&body)
        .map_err(|_| Error::BadRequest("webhook body is not UTF-8".into()))?;
    let event: Value =
        serde_json::from_str(text).map_err(|_| Error::BadRequest("invalid webhook JSON".into()))?;
    if let Some(change) = plan_change_from_event(&event, &billing.price_id_pro) {
        apply_plan_change(&state.pool, change).await?;
    }
    Ok((StatusCode::OK, Json(json!({ "received": true }))))
}

async fn ensure_customer(
    state: &AppState,
    billing: &crate::config::StripeBilling,
    user: &User,
) -> Result<String, Error> {
    if let Some(id) = user.stripe_customer_id.as_deref().filter(|s| !s.is_empty()) {
        return Ok(id.to_string());
    }
    let id = stripe_client::create_customer(billing, user.id, &user.email).await?;
    db::set_stripe_customer_id(&state.pool, user.id, &id).await?;
    Ok(id)
}

/// Applies a verified Stripe plan change to Postgres.
pub async fn apply_plan_change(pool: &sqlx::PgPool, change: PlanChange) -> Result<(), Error> {
    match change {
        PlanChange::Pro {
            user_id,
            ref customer_id,
        } => {
            let uid = resolve_user(pool, user_id, Some(customer_id)).await?;
            db::set_user_plan_and_stripe_customer(pool, uid, "pro", customer_id).await?;
        }
        PlanChange::Free {
            user_id,
            customer_id,
        } => {
            let uid = resolve_user(pool, user_id, customer_id.as_deref()).await?;
            db::set_user_plan(pool, uid, "free").await?;
        }
    }
    Ok(())
}

async fn resolve_user(
    pool: &sqlx::PgPool,
    user_id: Option<Uuid>,
    customer_id: Option<&str>,
) -> Result<Uuid, Error> {
    if let Some(id) = user_id {
        return Ok(id);
    }
    let Some(cid) = customer_id.filter(|s| !s.is_empty()) else {
        return Err(Error::BadRequest(
            "webhook event missing user_id and customer".into(),
        ));
    };
    db::user_by_stripe_customer(pool, cid)
        .await?
        .map(|u| u.id)
        .ok_or_else(|| Error::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn checkout_event_maps_to_pro_change() {
        let user = Uuid::new_v4();
        let event = json!({
            "type": "checkout.session.completed",
            "data": { "object": {
                "mode": "subscription",
                "payment_status": "paid",
                "client_reference_id": user.to_string(),
                "customer": "cus_x",
            }}
        });
        let change = plan_change_from_event(&event, "price_pro").expect("change");
        assert_eq!(plan_str(&change), "pro");
        match change {
            PlanChange::Pro {
                user_id,
                customer_id,
            } => {
                assert_eq!(user_id, Some(user));
                assert_eq!(customer_id, "cus_x");
            }
            _ => panic!("expected Pro"),
        }
    }
}
