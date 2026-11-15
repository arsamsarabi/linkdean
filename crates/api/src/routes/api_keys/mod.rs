use crate::auth::{AuthUser, SessionUser};
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::{CreateApiKey, CreateApiKeyResponse};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_keys, create_key))
        .routes(routes!(revoke_key))
}

#[utoipa::path(
    get,
    operation_id = "api_keys_list_keys",
    path = "/api-keys",
    tag = "api-keys",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list_keys(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    db::require_api_keys_allowed(&state.pool, &user).await?;
    let keys = db::list_api_keys(&state.pool, user.id).await?;
    Ok(Json(json!({ "keys": keys })))
}

/// Login sessions only: a key that could mint keys would survive its own revocation.
#[utoipa::path(
    post,
    operation_id = "api_keys_create_key",
    path = "/api-keys",
    tag = "api-keys",
    responses(
        (status = 200, description = "OK", body = CreateApiKeyResponse),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("session_token" = []), ("session_cookie" = [])),
)]
async fn create_key(
    State(state): State<AppState>,
    SessionUser(user): SessionUser,
    Json(req): Json<CreateApiKey>,
) -> ApiResult<Json<CreateApiKeyResponse>> {
    let created = db::create_api_key(&state.pool, &user, req).await?;
    Ok(Json(created))
}

#[utoipa::path(
    delete,
    operation_id = "api_keys_revoke_key",
    path = "/api-keys/{id}",
    tag = "api-keys",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn revoke_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    // Revoke always allowed for own keys (even if gates later flip off)
    db::revoke_api_key(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
