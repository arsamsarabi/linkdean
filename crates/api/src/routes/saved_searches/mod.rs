use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::{CreateSavedSearch, PatchSavedSearch, SavedSearchView};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(patch, delete))
}

#[utoipa::path(
    get,
    operation_id = "saved_searches_list",
    path = "/saved-searches",
    tag = "saved-searches",
    responses(
        (status = 200, description = "OK", body = Vec<SavedSearchView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<SavedSearchView>>> {
    Ok(Json(db::list_saved_searches(&state.pool, user.id).await?))
}

#[utoipa::path(
    post,
    operation_id = "saved_searches_create",
    path = "/saved-searches",
    tag = "saved-searches",
    responses(
        (status = 200, description = "OK", body = SavedSearchView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateSavedSearch>,
) -> ApiResult<Json<SavedSearchView>> {
    Ok(Json(
        db::create_saved_search(&state.pool, user.id, req).await?,
    ))
}

#[utoipa::path(
    patch,
    operation_id = "saved_searches_patch",
    path = "/saved-searches/{id}",
    tag = "saved-searches",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = SavedSearchView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn patch(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchSavedSearch>,
) -> ApiResult<Json<SavedSearchView>> {
    Ok(Json(
        db::patch_saved_search(&state.pool, user.id, id, req).await?,
    ))
}

#[utoipa::path(
    delete,
    operation_id = "saved_searches_delete",
    path = "/saved-searches/{id}",
    tag = "saved-searches",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_saved_search(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}
