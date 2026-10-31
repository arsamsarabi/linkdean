use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use linkdean_core::{Category, CreateCategory, UpdateCategory};
use linkdean_db::{self as db};
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list).post(create))
        .route("/categories/{id}", patch(update).delete(delete_one))
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub include_vault: bool,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<Category>>> {
    let include_vault = q.include_vault && state.vault_key(user.id).await.is_some();
    let cats = db::list_categories(&state.pool, user.id, q.include_hidden, include_vault).await?;
    Ok(Json(cats))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateCategory>,
) -> ApiResult<Json<Category>> {
    if req.is_vault && state.vault_key(user.id).await.is_none() {
        // allow creating vault category only if vault setup exists; unlock not strictly required to create empty cat
        if !db::user_has_vault(&state.pool, user.id).await? {
            return Err(linkdean_core::Error::BadRequest(
                "setup vault before creating vault categories".into(),
            )
            .into());
        }
    }
    let cat = db::create_category(&state.pool, user.id, req).await?;
    Ok(Json(cat))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategory>,
) -> ApiResult<Json<Category>> {
    let cat = db::update_category(&state.pool, user.id, id, req).await?;
    Ok(Json(cat))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::delete_category(&state.pool, user.id, id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
