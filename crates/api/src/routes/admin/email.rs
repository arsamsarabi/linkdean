//! Settings → Admin → Email: configuration status, delivery counts and a test send.

use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use linkdean_core::Error;
use linkdean_db::{self as db};
use linkdean_mail::MailConfig;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};

/// Test sends per admin per rate-limit window (15 minutes).
const TEST_SENDS: u32 = 5;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(send_test))
}

/// What an admin may see of the SMTP settings. Never the username or password.
fn config_json(cfg: &MailConfig) -> Value {
    json!({
        "host": cfg.host,
        "port": cfg.port,
        "tls": cfg.tls.as_str(),
        "from": cfg.from.to_string(),
    })
}

#[utoipa::path(
    get,
    operation_id = "admin_email_status",
    path = "/admin/email",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn status(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let stats = db::email_stats(&state.pool).await?;
    let config = state.mail.as_deref().map(|m| config_json(&m.cfg));
    Ok(Json(
        json!({ "enabled": config.is_some(), "config": config, "stats": stats }),
    ))
}

/// Sends straight away, skipping the outbox, so the admin sees the SMTP result at once.
#[utoipa::path(
    post,
    operation_id = "admin_email_send_test",
    path = "/admin/email/test",
    tag = "admin",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn send_test(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
) -> ApiResult<Json<Value>> {
    let Some(mail) = state.mail.as_deref() else {
        return Err(Error::BadRequest("mail is off: set SMTP_HOST and MAIL_FROM".into()).into());
    };
    let limits = [(format!("mail-test:{}", admin.id), TEST_SENDS)];
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let rendered = linkdean_mail::render("test", &json!({ "recipient": admin.email }), &mail.site)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(Json(
        match mail.mailer.send(&admin.email, &rendered).await {
            Ok(()) => json!({ "ok": true, "to": admin.email }),
            Err(error) => json!({ "ok": false, "to": admin.email, "error": error }),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_json_never_shows_credentials() {
        let cfg = MailConfig::from_vars(|k| {
            match k {
                "SMTP_HOST" => Some("smtp.example.com"),
                "SMTP_USERNAME" => Some("bot"),
                "SMTP_PASSWORD" => Some("hunter2"),
                "MAIL_FROM" => Some("Linkdean <a@example.com>"),
                _ => None,
            }
            .map(String::from)
        })
        .unwrap()
        .unwrap();
        assert!(cfg.credentials.is_some());
        let json = config_json(&cfg);
        let text = json.to_string();
        assert!(!text.contains("hunter2"), "{text}");
        assert!(!text.to_lowercase().contains("password"), "{text}");
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["from", "host", "port", "tls"]);
    }
}
