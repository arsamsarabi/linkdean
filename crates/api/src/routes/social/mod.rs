use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use linkdean_core::{CreateSocialComment, SocialSubjectType, UpsertSocialRating};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(summary))
        .routes(routes!(like, unlike))
        .routes(routes!(comment))
        .routes(routes!(rate, unrate))
        .routes(routes!(delete_comment))
}

fn parse_subject(s: &str) -> Result<SocialSubjectType, linkdean_core::Error> {
    SocialSubjectType::parse(s).map_err(linkdean_core::Error::BadRequest)
}

#[utoipa::path(
    get,
    operation_id = "social_summary",
    path = "/social/{subject_type}/{subject_id}",
    tag = "social",
    params(("subject_type" = String, Path), ("subject_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn summary(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::get_social_summary(&state.pool, user.id, st, subject_id).await?;
    Ok(Json(json!({ "social": summary })))
}

#[utoipa::path(
    post,
    operation_id = "social_like",
    path = "/social/{subject_type}/{subject_id}/like",
    tag = "social",
    params(("subject_type" = String, Path), ("subject_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn like(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::set_like(&state.pool, user.id, st, subject_id, true).await?;
    Ok(Json(json!({ "social": summary })))
}

#[utoipa::path(
    delete,
    operation_id = "social_unlike",
    path = "/social/{subject_type}/{subject_id}/like",
    tag = "social",
    params(("subject_type" = String, Path), ("subject_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unlike(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::set_like(&state.pool, user.id, st, subject_id, false).await?;
    Ok(Json(json!({ "social": summary })))
}

#[utoipa::path(
    post,
    operation_id = "social_comment",
    path = "/social/{subject_type}/{subject_id}/comments",
    tag = "social",
    params(("subject_type" = String, Path), ("subject_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn comment(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
    Json(req): Json<CreateSocialComment>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::add_comment(&state.pool, user.id, st, subject_id, req).await?;
    Ok(Json(json!({ "social": summary })))
}

#[utoipa::path(
    delete,
    operation_id = "social_delete_comment",
    path = "/social/comments/{id}",
    tag = "social",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_comment(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_comment(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "social_rate",
    path = "/social/{subject_type}/{subject_id}/rating",
    tag = "social",
    params(("subject_type" = String, Path), ("subject_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn rate(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
    Json(req): Json<UpsertSocialRating>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::upsert_rating(&state.pool, user.id, st, subject_id, req).await?;
    Ok(Json(json!({ "social": summary })))
}

#[utoipa::path(
    delete,
    operation_id = "social_unrate",
    path = "/social/{subject_type}/{subject_id}/rating",
    tag = "social",
    params(("subject_type" = String, Path), ("subject_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unrate(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::clear_rating(&state.pool, user.id, st, subject_id).await?;
    Ok(Json(json!({ "social": summary })))
}
