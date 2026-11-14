use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use linkdean_core::{ActivityAction, ActivitySubjectType, ListActivityFilter};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    limit: Option<i64>,
    subject_type: Option<String>,
    action: Option<String>,
}

#[utoipa::path(
    get,
    operation_id = "activity_list",
    path = "/activity",
    tag = "activity",
    params(ListQuery),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let subject_type = match q.subject_type.as_deref() {
        None | Some("") => None,
        Some(s) => Some(ActivitySubjectType::parse(s).map_err(linkdean_core::Error::BadRequest)?),
    };
    let action = match q.action.as_deref() {
        None | Some("") => None,
        Some(s) => Some(ActivityAction::parse(s).map_err(linkdean_core::Error::BadRequest)?),
    };
    let filter = ListActivityFilter {
        subject_type,
        action,
        limit: q.limit.unwrap_or(50),
    };
    let items = db::list_activity(&state.pool, user.id, filter).await?;
    Ok(Json(json!({ "events": items })))
}
