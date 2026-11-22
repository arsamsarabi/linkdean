use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::notification_mail::{self, InviteEmail};
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Path, Query, State};
use axum::Json;
use linkdean_core::{
    CategoryPublicLinkView, CategoryShareView, CategorySharesEnvelope, CategoryView,
    CreateCategory, CreateCategoryShare, CreateCategoryShareOutcome, ForkCategory, MoveToTeam,
    PendingShareInviteView, PendingShareSubjectType, UpdateCategory, UpdateCategoryShare,
    UpdatePendingShareInvite,
};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(update, delete_one))
        .routes(routes!(fork))
        .routes(routes!(move_to_team))
        .routes(routes!(copy_to_personal))
        .routes(routes!(list_shares, create_share))
        .routes(routes!(update_share, delete_share))
        .routes(routes!(update_pending_share, delete_pending_share))
        .routes(routes!(
            get_public_link,
            enable_public_link,
            disable_public_link
        ))
        .routes(routes!(rotate_public_link))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub include_vault: bool,
    /// When set, list this team space instead of the personal library.
    #[serde(default)]
    pub team_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    operation_id = "categories_list",
    path = "/categories",
    tag = "categories",
    params(ListQuery),
    responses(
        (status = 200, description = "OK", body = Vec<CategoryView>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<CategoryView>>> {
    if let Some(team_id) = q.team_id {
        let settings = db::get_user_settings(&state.pool, user.id).await?;
        let cats = db::list_team_categories(
            &state.pool,
            user.id,
            team_id,
            q.include_hidden,
            settings.show_deleted_in_library,
        )
        .await?;
        return Ok(Json(cats));
    }
    let unlocked = if q.include_vault {
        state
            .vault_keyring(&vault_session, user.id)
            .await
            .vault_ids()
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

#[utoipa::path(
    post,
    operation_id = "categories_create",
    path = "/categories",
    tag = "categories",
    responses(
        (status = 200, description = "OK", body = CategoryView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateCategory>,
) -> ApiResult<Json<CategoryView>> {
    let cat = db::create_category(&state.pool, user.id, req).await?;
    Ok(Json(cat))
}

#[utoipa::path(
    patch,
    operation_id = "categories_update",
    path = "/categories/{id}",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn update(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategory>,
) -> ApiResult<Json<CategoryView>> {
    let unlocked = state
        .vault_keyring(&vault_session, user.id)
        .await
        .vault_ids();
    db::require_category_unlocked(&state.pool, user.id, id, &unlocked).await?;
    let cat = db::update_category(&state.pool, user.id, id, req).await?;
    Ok(Json(cat))
}

#[utoipa::path(
    delete,
    operation_id = "categories_delete_one",
    path = "/categories/{id}",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_category_by_mode(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "categories_fork",
    path = "/categories/{id}/fork",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn fork(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ForkCategory>,
) -> ApiResult<Json<Value>> {
    let (cat, copied) = db::fork_category(&state.pool, user.id, id, req).await?;
    let bookmarks_copied = copied.len();
    for view in &copied {
        if let Err(e) = crate::search_index::upsert(&state, view).await {
            tracing::warn!(bookmark_id = %view.id, error = %e, "meili upsert after fork failed");
        }
    }
    Ok(Json(json!({
        "category": cat,
        "bookmarks_copied": bookmarks_copied,
    })))
}

#[utoipa::path(
    post,
    operation_id = "categories_move_to_team",
    path = "/categories/{id}/move-to-team",
    tag = "categories",
    params(("id" = Uuid, Path)),
    request_body = MoveToTeam,
    responses(
        (status = 200, description = "OK", body = CategoryView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn move_to_team(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<MoveToTeam>,
) -> ApiResult<Json<CategoryView>> {
    let cat = db::move_category_to_team(&state.pool, user.id, id, req).await?;
    Ok(Json(cat))
}

#[utoipa::path(
    post,
    operation_id = "categories_copy_to_personal",
    path = "/categories/{id}/copy-to-personal",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn copy_to_personal(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryView>> {
    let cat = db::copy_category_to_personal(&state.pool, user.id, id).await?;
    Ok(Json(cat))
}

#[utoipa::path(
    get,
    operation_id = "categories_list_shares",
    path = "/categories/{id}/shares",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategorySharesEnvelope),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    post,
    operation_id = "categories_create_share",
    path = "/categories/{id}/shares",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCategoryShare>,
) -> ApiResult<Json<Value>> {
    let since = notification_mail::invite_clock(&state).await;
    let outcome = db::create_share(&state.pool, user.id, id, req).await?;
    let email = match &outcome {
        CreateCategoryShareOutcome::Pending { invite } => {
            notification_mail::invite_email(&state, &user, invite, since).await
        }
        CreateCategoryShareOutcome::Active { .. } => InviteEmail::default(),
    };
    Ok(Json(notification_mail::with_invite_email(&outcome, email)))
}

#[utoipa::path(
    patch,
    operation_id = "categories_update_share",
    path = "/categories/{id}/shares/{user_id}",
    tag = "categories",
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryShareView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn update_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateCategoryShare>,
) -> ApiResult<Json<CategoryShareView>> {
    let share = db::update_share(&state.pool, user.id, id, invitee_user_id, req).await?;
    Ok(Json(share))
}

#[utoipa::path(
    delete,
    operation_id = "categories_delete_share",
    path = "/categories/{id}/shares/{user_id}",
    tag = "categories",
    params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn delete_share(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((id, invitee_user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    db::delete_share(&state.pool, user.id, id, invitee_user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    patch,
    operation_id = "categories_update_pending_share",
    path = "/categories/{id}/pending-shares/{pending_id}",
    tag = "categories",
    params(("id" = Uuid, Path), ("pending_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = PendingShareInviteView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    delete,
    operation_id = "categories_delete_pending_share",
    path = "/categories/{id}/pending-shares/{pending_id}",
    tag = "categories",
    params(("id" = Uuid, Path), ("pending_id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
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

#[utoipa::path(
    get,
    operation_id = "categories_get_public_link",
    path = "/categories/{id}/public-link",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryPublicLinkView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn get_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::get_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}

#[utoipa::path(
    post,
    operation_id = "categories_enable_public_link",
    path = "/categories/{id}/public-link",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryPublicLinkView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn enable_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::enable_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}

#[utoipa::path(
    post,
    operation_id = "categories_rotate_public_link",
    path = "/categories/{id}/public-link/rotate",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryPublicLinkView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn rotate_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::rotate_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}

#[utoipa::path(
    delete,
    operation_id = "categories_disable_public_link",
    path = "/categories/{id}/public-link",
    tag = "categories",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = CategoryPublicLinkView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn disable_public_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CategoryPublicLinkView>> {
    let link = db::disable_category_public_link(&state.pool, user.id, id).await?;
    Ok(Json(link))
}
