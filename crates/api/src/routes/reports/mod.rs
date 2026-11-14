//! Signed-in users report public lists, profiles and comments; admins work the queue under
//! `/admin/reports` (see `admin::reports`).

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::rate_limit::report_limit;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use linkdean_core::CreateAbuseReport;
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_report))
        .routes(routes!(report_config))
}

#[utoipa::path(
    post,
    operation_id = "reports_create_report",
    path = "/reports",
    tag = "reports",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_report(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateAbuseReport>,
) -> ApiResult<Json<Value>> {
    let limits = report_limit(user.id);
    state.limiter.check(&limits)?;
    state.limiter.record(&limits);
    let report = db::create_report(&state.pool, user.id, &req).await?;
    tracing::info!(user_id = %user.id, report_id = %report.id, target = %report.target_type, "abuse report filed");
    // The reporter's view: no reporter or admin fields beyond what they sent.
    Ok(Json(json!({ "ok": true, "id": report.id })))
}

/// Where to send abuse and takedown requests that do not fit the report form.
#[utoipa::path(
    get,
    operation_id = "reports_report_config",
    path = "/reports/config",
    tag = "reports",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn report_config(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
) -> ApiResult<Json<Value>> {
    let platform = db::get_platform_settings(&state.pool).await?;
    Ok(Json(
        json!({ "abuse_contact_email": platform.abuse_contact_email }),
    ))
}
