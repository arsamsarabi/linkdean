use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use linkdean_core::{
    CategoryPublicLinkView, CategoryShareView, CategorySharesEnvelope, CategoryView,
    CreateCategory, CreateCategoryShare, CreateCategoryShareOutcome, ForkCategory,
    PendingShareInviteView, PendingShareSubjectType, UpdateCategory, UpdateCategoryShare,
    UpdatePendingShareInvite,
};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list).post(create))
        .route("/categories/{id}", patch(update).delete(delete_one))
        .route("/categories/{id}/fork", post(fork))
        .route(
            "/categories/{id}/shares",
            get(list_shares).post(create_share),
        )
        .route(
            "/categories/{id}/shares/{user_id}",
            patch(update_share).delete(delete_share),
        )
        .route(
            "/categories/{id}/pending-shares/{pending_id}",
            patch(update_pending_share).delete(delete_pending_share),
        )
        .route(
            "/categories/{id}/public-link",
            get(get_public_link)
                .post(enable_public_link)
                .delete(disable_public_link),
        )
        .route(
            "/categories/{id}/public-link/rotate",
            post(rotate_public_link),
        )
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
) -> ApiResult<Json<Vec<CategoryView>>> {
    let unlocked = if q.include_vault {
        state.vault_keyring(user.id).await.vault_ids()
    } else {
        Vec::new()
    };
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let cats = db::list_categories(
        &state.pool,
        user.id,
        q.include_hidden,
        &unlocked,
        settings.show_deleted_in_library,
    )
    .await?;
    Ok(Json(cats))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateCategory>,
) -> ApiResult<Json<CategoryView>> {
    if req.is_vault
        && state.vault_keyring(user.id).await.is_empty()
        && !db::user_has_vault(&state.pool, user.id).await?
    {
        return Err(linkdean_core::Error::BadRequest(
            "setup vault before creating vault categories".into(),
        )
        .into());
    }
    let cat = db::create_category(&state.pool, user.id, req).await?;
    Ok(Json(cat))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategory>,
) -> ApiResult<Json<CategoryView>> {
    let cat = db::update_category(&state.pool, user.id, id, req).await?;
    Ok(Json(cat))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_category(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn fork(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ForkCategory>,
) -> ApiResult<Json<Value>> {
    let (cat, copied) = db::fork_category(&state.pool, user.id, id, req).await?;
    let bookmarks_copied = copied.len();
    for view in &copied {
        if let Err(e) = state.search.upsert_bookmark(view).await {
            tracing::warn!(bookmark_id = %view.id, error = %e, "meili upsert after fork failed");
        }
    }
    Ok(Json(json!({
        "category": cat,
        "bookmarks_copied": bookmarks_copied,
    })))
}

async fn list_shares(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategorySharesEnvelope>> {
    let shares = db::list_shares_for_category(&state.pool, user.id, id).await?;
    let pending =
        db::list_pending_for_subject(&state.pool, user.id, PendingShareSubjectType::Category, id)
            .await?;
    Ok(Json(CategorySharesEnvelope { shares, pending }))
}

async fn create_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCategoryShare>,
) -> ApiResult<Json<CreateCategoryShareOutcome>> {
    let outcome = db::create_share(&state.pool, user.id, id, req).await?;
    Ok(Json(outcome))
}

async fn update_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateCategoryShare>,
) -> ApiResult<Json<CategoryShareView>> {
    let share = db::update_share(&state.pool, user.id, id, invitee_user_id, req).await?;
    Ok(Json(share))
}

async fn delete_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    db::delete_share(&state.pool, user.id, id, invitee_user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn update_pending_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, pending_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePendingShareInvite>,
) -> ApiResult<Json<PendingShareInviteView>> {
    let invite = db::update_pending_share_invite(
        &state.pool,
        user.id,
        PendingShareSubjectType::Category,
        id,
        pending_id,
        req,
    )
    .await?;
    Ok(Json(invite))
}

async fn delete_pending_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, pending_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    db::revoke_pending_share_invite(
        &state.pool,
        user.id,
        PendingShareSubjectType::Category,
        id,
        pending_id,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn get_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::get_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}

async fn enable_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::enable_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}

async fn rotate_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::rotate_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}

async fn disable_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::disable_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}
