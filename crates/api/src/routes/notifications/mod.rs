use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list))
        .route("/notifications/unread-count", get(unread_count))
        .route("/notifications/read-all", post(read_all))
        .route("/notifications/{id}/read", post(read_one))
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let items = db::list_notifications(&state.pool, user.id, q.limit.unwrap_or(50)).await?;
    Ok(Json(json!({ "notifications": items })))
}

async fn unread_count(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let count = db::unread_notification_count(&state.pool, user.id).await?;
    Ok(Json(json!({ "count": count })))
}

async fn read_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let n = db::mark_notification_read(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "notification": n })))
}

async fn read_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let updated = db::mark_all_notifications_read(&state.pool, user.id).await?;
    Ok(Json(json!({ "ok": true, "updated": updated })))
}
