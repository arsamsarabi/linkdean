use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::{CreateSocialComment, SocialSubjectType, UpsertSocialRating};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/social/{subject_type}/{subject_id}", get(summary))
        .route(
            "/social/{subject_type}/{subject_id}/like",
            post(like).delete(unlike),
        )
        .route(
            "/social/{subject_type}/{subject_id}/comments",
            post(comment),
        )
        .route(
            "/social/{subject_type}/{subject_id}/rating",
            post(rate).delete(unrate),
        )
        .route(
            "/social/comments/{id}",
            axum::routing::delete(delete_comment),
        )
}

fn parse_subject(s: &str) -> Result<SocialSubjectType, linkdean_core::Error> {
    SocialSubjectType::parse(s).map_err(linkdean_core::Error::BadRequest)
}

async fn summary(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::get_social_summary(&state.pool, user.id, st, subject_id).await?;
    Ok(Json(json!({ "social": summary })))
}

async fn like(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::set_like(&state.pool, user.id, st, subject_id, true).await?;
    Ok(Json(json!({ "social": summary })))
}

async fn unlike(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::set_like(&state.pool, user.id, st, subject_id, false).await?;
    Ok(Json(json!({ "social": summary })))
}

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

async fn delete_comment(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_comment(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

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

async fn unrate(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((subject_type, subject_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let st = parse_subject(&subject_type)?;
    let summary = db::clear_rating(&state.pool, user.id, st, subject_id).await?;
    Ok(Json(json!({ "social": summary })))
}
