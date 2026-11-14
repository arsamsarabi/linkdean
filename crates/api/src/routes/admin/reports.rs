//! Settings → Admin → Reports: the abuse report queue.

use crate::auth::AdminUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use linkdean_core::{AbuseAction, ResolveAbuseReport};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_reports))
        .routes(routes!(resolve_report))
}

#[derive(serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    status: Option<String>,
}

#[utoipa::path(
    get,
    operation_id = "admin_reports_list_reports",
    path = "/admin/reports",
    tag = "admin",
    params(ListQuery),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn list_reports(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let reports = db::list_reports(&state.pool, q.status.as_deref().unwrap_or("open")).await?;
    Ok(Json(json!({ "reports": reports })))
}

#[utoipa::path(
    post,
    operation_id = "admin_reports_resolve_report",
    path = "/admin/reports/{id}/resolve",
    tag = "admin",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn resolve_report(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveAbuseReport>,
) -> ApiResult<Json<Value>> {
    let report = db::resolve_report(&state.pool, admin.id, id, &req).await?;
    if let Some(user_id) = report.target_user_id {
        match req.action {
            AbuseAction::Ban => state.clear_vault_keys(user_id).await,
            AbuseAction::Warn => warn_user(&state, user_id, &report).await,
            AbuseAction::Dismiss | AbuseAction::Hide => {}
        }
    }
    Ok(Json(json!({ "report": report })))
}

/// Emails the warning when mail is on. Best effort: the resolution is already recorded, so a
/// failed send is logged, not returned.
async fn warn_user(state: &AppState, user_id: Uuid, report: &linkdean_core::AbuseReport) {
    let Some(note) = report.resolution_note.as_deref() else {
        return;
    };
    let user = match db::find_user_by_id(&state.pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!(%user_id, error = %e, "abuse warning: user lookup failed");
            return;
        }
    };
    let data = json!({ "subject": report.target_label, "note": note });
    if let Err(e) = crate::mail::enqueue(state, &user.email, "abuse_warning", &data).await {
        tracing::warn!(%user_id, error = %e, "abuse warning email failed");
    }
}
