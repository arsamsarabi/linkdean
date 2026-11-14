use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(unread_count))
        .routes(routes!(read_all))
        .routes(routes!(read_one))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct ListQuery {
    limit: Option<i64>,
}

#[utoipa::path(
    get,
    operation_id = "notifications_list",
    path = "/notifications",
    tag = "notifications",
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
    let items = db::list_notifications(&state.pool, user.id, q.limit.unwrap_or(50)).await?;
    Ok(Json(json!({ "notifications": items })))
}

#[utoipa::path(
    get,
    operation_id = "notifications_unread_count",
    path = "/notifications/unread-count",
    tag = "notifications",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unread_count(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let count = db::unread_notification_count(&state.pool, user.id).await?;
    Ok(Json(json!({ "count": count })))
}

#[utoipa::path(
    post,
    operation_id = "notifications_read_one",
    path = "/notifications/{id}/read",
    tag = "notifications",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn read_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let n = db::mark_notification_read(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "notification": n })))
}

#[utoipa::path(
    post,
    operation_id = "notifications_read_all",
    path = "/notifications/read-all",
    tag = "notifications",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn read_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let updated = db::mark_all_notifications_read(&state.pool, user.id).await?;
    Ok(Json(json!({ "ok": true, "updated": updated })))
}
