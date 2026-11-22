//! Thin Stripe REST helpers for Checkout and the Customer Portal (form-urlencoded API).

use crate::config::StripeBilling;
use linkdean_core::{Error, Result};
use serde::Deserialize;
use uuid::Uuid;

const API: &str = "https://api.stripe.com/v1";

#[derive(Debug, Deserialize)]
struct IdUrl {
    id: String,
    #[serde(default)]
    url: Option<String>,
}

/// Creates a Stripe Customer for `user_id` / `email`.
pub async fn create_customer(
    billing: &StripeBilling,
    user_id: Uuid,
    email: &str,
) -> Result<String> {
    let body = form(&[
        ("email", email),
        ("metadata[user_id]", &user_id.to_string()),
    ]);
    let created: IdUrl = post(billing, "/customers", &body).await?;
    Ok(created.id)
}

/// Opens a subscription Checkout Session for personal Pro. Returns the hosted URL.
pub async fn create_checkout_session(
    billing: &StripeBilling,
    app_url: &str,
    user_id: Uuid,
    customer_id: &str,
) -> Result<String> {
    let success = format!("{app_url}/settings?billing=success");
    let cancel = format!("{app_url}/settings?billing=cancel");
    let user = user_id.to_string();
    let body = form(&[
        ("mode", "subscription"),
        ("customer", customer_id),
        ("client_reference_id", &user),
        ("success_url", &success),
        ("cancel_url", &cancel),
        ("line_items[0][price]", &billing.price_id_pro),
        ("line_items[0][quantity]", "1"),
        ("subscription_data[metadata][user_id]", &user),
        ("metadata[user_id]", &user),
    ]);
    let session: IdUrl = post(billing, "/checkout/sessions", &body).await?;
    session
        .url
        .ok_or_else(|| Error::msg("Stripe Checkout Session missing url"))
}

/// Opens the Stripe Customer Portal. Returns the hosted URL.
pub async fn create_portal_session(
    billing: &StripeBilling,
    app_url: &str,
    customer_id: &str,
) -> Result<String> {
    let return_url = format!("{app_url}/settings");
    let body = form(&[("customer", customer_id), ("return_url", &return_url)]);
    let session: IdUrl = post(billing, "/billing_portal/sessions", &body).await?;
    session
        .url
        .ok_or_else(|| Error::msg("Stripe Customer Portal session missing url"))
}

fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding_encode(k), urlencoding_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Minimal application/x-www-form-urlencoded encode (Stripe keys are simple).
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

async fn post<T: for<'de> Deserialize<'de>>(
    billing: &StripeBilling,
    path: &str,
    body: &str,
) -> Result<T> {
    let url = format!("{API}{path}");
    let res = reqwest::Client::new()
        .post(&url)
        .bearer_auth(&billing.secret_key)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, path, "stripe request failed");
            Error::msg("billing provider unreachable")
        })?;
    let status = res.status();
    let text = res.text().await.map_err(|e| Error::msg(e.to_string()))?;
    if !status.is_success() {
        tracing::warn!(%status, path, body = %text, "stripe error");
        return Err(Error::msg("billing provider refused the request"));
    }
    serde_json::from_str(&text).map_err(|e| {
        tracing::warn!(error = %e, path, "stripe response parse");
        Error::msg("billing provider returned unexpected JSON")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_encodes_spaces_and_brackets() {
        let s = form(&[("metadata[user_id]", "a b"), ("q", "1")]);
        assert!(s.contains("metadata%5Buser_id%5D=a+b"));
        assert!(s.contains("q=1"));
    }
}
