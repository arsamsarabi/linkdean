//! Cloudflare Turnstile Siteverify for registration.

use linkdean_core::Error;
use serde::Deserialize;
use std::net::IpAddr;

pub const SITEVERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[derive(Debug, Deserialize)]
struct SiteverifyResponse {
    success: bool,
}

/// Validate a widget token. Fail closed on network or API failure.
pub async fn verify(
    client: &reqwest::Client,
    siteverify_url: &str,
    secret: &str,
    token: &str,
    remote_ip: Option<IpAddr>,
) -> Result<(), Error> {
    let token = token.trim();
    if token.is_empty() {
        return Err(Error::BadRequest("complete the captcha to register".into()));
    }
    let mut form = vec![
        ("secret", secret.to_string()),
        ("response", token.to_string()),
    ];
    if let Some(ip) = remote_ip {
        form.push(("remoteip", ip.to_string()));
    }
    let res = client
        .post(siteverify_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "turnstile siteverify request failed");
            Error::BadRequest("captcha check failed; try again".into())
        })?;
    if !res.status().is_success() {
        tracing::warn!(status = %res.status(), "turnstile siteverify bad status");
        return Err(Error::BadRequest("captcha check failed; try again".into()));
    }
    let body: SiteverifyResponse = res.json().await.map_err(|e| {
        tracing::warn!(error = %e, "turnstile siteverify body");
        Error::BadRequest("captcha check failed; try again".into())
    })?;
    if !body.success {
        return Err(Error::BadRequest("captcha check failed; try again".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use axum::{Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn spawn_siteverify(success: bool) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/siteverify",
            post(move || async move { Json(json!({ "success": success })) }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}/siteverify");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        // Give the server a beat.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        (url, handle)
    }

    #[tokio::test]
    async fn accepts_a_successful_siteverify() {
        let (url, _h) = spawn_siteverify(true).await;
        let client = reqwest::Client::new();
        verify(
            &client,
            &url,
            "secret",
            "token",
            Some("1.2.3.4".parse::<IpAddr>().unwrap()),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn rejects_a_failed_siteverify() {
        let (url, _h) = spawn_siteverify(false).await;
        let client = reqwest::Client::new();
        let err = verify(&client, &url, "secret", "token", None)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    #[tokio::test]
    async fn rejects_an_empty_token() {
        let client = reqwest::Client::new();
        let err = verify(&client, "http://127.0.0.1:9/nope", "secret", "  ", None)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }
}
