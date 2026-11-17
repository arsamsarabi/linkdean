use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::rate_limit::vault_limit;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use linkdean_core::{
    ChangeGlobalPassphraseRequest, CreateVaultRequest, DeleteVaultRequest, Error,
    MoveToVaultRequest, RecoverVaultRequest, RenameVaultRequest, Vault, VaultKeyring,
    VaultMoveOutcome, VaultUnlockRequest, VaultView,
};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(lock_all))
        .routes(routes!(move_items))
        .routes(routes!(
            global_status,
            global_enable,
            global_change,
            global_disable
        ))
        .routes(routes!(global_unlock))
        .routes(routes!(rename, delete_one))
        .routes(routes!(unlock_one))
        .routes(routes!(lock_one))
        .routes(routes!(recover))
        .routes(routes!(new_recovery_key))
        .routes(routes!(global_join, global_leave))
}

fn check_passphrase(passphrase: &str) -> Result<(), crate::error::ApiError> {
    let min = linkdean_core::crypto::MIN_SECRET_LEN;
    if passphrase.len() < min {
        return Err(Error::BadRequest(format!("vault passphrase min {min} chars")).into());
    }
    Ok(())
}

async fn to_view(state: &AppState, keyring: &VaultKeyring, vault: Vault) -> ApiResult<VaultView> {
    let unlocked = keyring.contains(vault.id);
    let counts = if unlocked {
        Some(db::vault_item_counts(&state.pool, vault.id).await?)
    } else {
        None
    };
    Ok(VaultView {
        id: vault.id,
        name: vault.name,
        unlocked,
        has_recovery_key: vault.recovery_wrapped_key.is_some(),
        in_global: vault.global_wrapped_key.is_some(),
        category_count: counts.map(|c| c.0),
        bookmark_count: counts.map(|c| c.1),
    })
}

#[utoipa::path(
    get,
    operation_id = "vault_list",
    path = "/vaults",
    tag = "vaults",
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
    vault_session: VaultSession,
) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let mut vaults = Vec::new();
    for vault in db::list_vaults(&state.pool, user.id).await? {
        vaults.push(to_view(&state, &keyring, vault).await?);
    }
    Ok(Json(json!({ "vaults": vaults })))
}

#[utoipa::path(
    post,
    operation_id = "vault_create",
    path = "/vaults",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<CreateVaultRequest>,
) -> ApiResult<Json<Value>> {
    require_session(&vault_session)?;
    check_passphrase(&req.passphrase)?;
    let (vault, key, recovery_key) =
        db::create_vault(&state.pool, user.id, &req.name, &req.passphrase).await?;
    state
        .set_vault_key(&vault_session, user.id, vault.id, key)
        .await?;
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = to_view(&state, &keyring, vault).await?;
    Ok(Json(json!({ "vault": view, "recovery_key": recovery_key })))
}

#[utoipa::path(
    patch,
    operation_id = "vault_rename",
    path = "/vaults/{id}",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn rename(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameVaultRequest>,
) -> ApiResult<Json<Value>> {
    let vault = db::rename_vault(&state.pool, user.id, id, &req.name).await?;
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = to_view(&state, &keyring, vault).await?;
    Ok(Json(json!({ "vault": view })))
}

#[utoipa::path(
    delete,
    operation_id = "vault_delete_one",
    path = "/vaults/{id}",
    tag = "vaults",
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
    Json(req): Json<DeleteVaultRequest>,
) -> ApiResult<Json<Value>> {
    db::delete_vault(&state.pool, &user, id, &req.name, req.password.as_deref()).await?;
    state.clear_vault_key(user.id, id).await;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "vault_unlock_one",
    path = "/vaults/{id}/unlock",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn unlock_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    require_session(&vault_session)?;
    let limits = vault_limit(user.id);
    state.limiter.check(&limits)?;
    let result = db::unlock_vault(&state.pool, user.id, id, &req.passphrase).await;
    let key = state.limiter.settle(&limits, result)?;
    state
        .set_vault_key(&vault_session, user.id, id, key)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "vault_lock_one",
    path = "/vaults/{id}/lock",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn lock_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::get_user_vault(&state.pool, user.id, id).await?;
    state.clear_vault_key(user.id, id).await;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "vault_lock_all",
    path = "/vaults/lock-all",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn lock_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    state.clear_vault_keys(user.id).await;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "vault_recover",
    path = "/vaults/{id}/recover",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn recover(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<RecoverVaultRequest>,
) -> ApiResult<Json<Value>> {
    require_session(&vault_session)?;
    check_passphrase(&req.new_passphrase)?;
    let limits = vault_limit(user.id);
    state.limiter.check(&limits)?;
    let result = db::recover_vault(
        &state.pool,
        user.id,
        id,
        &req.recovery_key,
        &req.new_passphrase,
    )
    .await;
    let key = state.limiter.settle(&limits, result)?;
    state
        .set_vault_key(&vault_session, user.id, id, key)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

/// Replacing the recovery key cancels the old one, so it takes the vault passphrase even
/// when the vault is unlocked.
#[utoipa::path(
    post,
    operation_id = "vault_new_recovery_key",
    path = "/vaults/{id}/recovery-key",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn new_recovery_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let limits = vault_limit(user.id);
    state.limiter.check(&limits)?;
    let result = db::unlock_vault(&state.pool, user.id, id, &req.passphrase).await;
    let key = state.limiter.settle(&limits, result)?;
    let recovery_key = db::replace_recovery_key(&state.pool, user.id, id, &key).await?;
    Ok(Json(json!({ "recovery_key": recovery_key })))
}

#[utoipa::path(
    get,
    operation_id = "vault_global_status",
    path = "/vaults/global",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let enabled = db::has_global_passphrase(&state.pool, user.id).await?;
    Ok(Json(json!({ "enabled": enabled })))
}

#[utoipa::path(
    post,
    operation_id = "vault_global_enable",
    path = "/vaults/global",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_enable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.passphrase)?;
    db::enable_global_passphrase(&state.pool, user.id, &req.passphrase).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    put,
    operation_id = "vault_global_change",
    path = "/vaults/global",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_change(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<ChangeGlobalPassphraseRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.new_passphrase)?;
    db::change_global_passphrase(
        &state.pool,
        user.id,
        &req.current_passphrase,
        &req.new_passphrase,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    delete,
    operation_id = "vault_global_disable",
    path = "/vaults/global",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_disable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let limits = vault_limit(user.id);
    state.limiter.check(&limits)?;
    let result = db::disable_global_passphrase(&state.pool, user.id, &req.passphrase).await;
    state.limiter.settle(&limits, result)?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "vault_global_unlock",
    path = "/vaults/global/unlock",
    tag = "vaults",
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_unlock(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    require_session(&vault_session)?;
    let limits = vault_limit(user.id);
    state.limiter.check(&limits)?;
    let result = db::unlock_with_global(&state.pool, user.id, &req.passphrase).await;
    let opened = state.limiter.settle(&limits, result)?;
    let mut ids = Vec::with_capacity(opened.len());
    for (id, key) in opened {
        state
            .set_vault_key(&vault_session, user.id, id, key)
            .await?;
        ids.push(id);
    }
    Ok(Json(json!({ "unlocked": ids })))
}

#[utoipa::path(
    post,
    operation_id = "vault_global_join",
    path = "/vaults/{id}/global",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_join(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let key = keyring.get(id).ok_or(Error::VaultLocked)?;
    db::join_global(&state.pool, user.id, id, key).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    delete,
    operation_id = "vault_global_leave",
    path = "/vaults/{id}/global",
    tag = "vaults",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK", body = serde_json::Value),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn global_leave(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::leave_global(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[utoipa::path(
    post,
    operation_id = "vault_move_items",
    path = "/vaults/move",
    tag = "vaults",
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn move_items(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<MoveToVaultRequest>,
) -> ApiResult<Response> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let outcome = db::move_to_vault(&state.pool, user.id, req, &keyring).await?;
    let (left_search, joined_search) = match outcome {
        VaultMoveOutcome::Blocked(blockers) => {
            return Ok(
                crate::error::ErrorBody::new("vault_move_blocked", "This move is blocked.")
                    .with_details(json!({ "blockers": blockers }))
                    .into_response_with(StatusCode::CONFLICT),
            );
        }
        VaultMoveOutcome::Moved {
            left_search,
            joined_search,
        } => (left_search, joined_search),
    };

    for id in left_search {
        if let Err(e) = state.search.delete_bookmark(id).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili delete failed");
        }
        linkdean_worker::purge_preview_assets(std::path::Path::new(&state.cfg.preview_dir), id)
            .await;
    }
    for id in joined_search {
        let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
        if let Some(view) = db::bookmark_to_view(&state.pool, raw, &keyring, user.id).await? {
            if let Err(e) = state.search.upsert_bookmark(&view).await {
                tracing::warn!(bookmark_id = %id, error = %e, "meili upsert failed");
            }
        }
        let pool = state.pool.clone();
        let search = state.search.clone();
        let opts = state.enrich_opts();
        let uid = user.id;
        tokio::spawn(async move {
            linkdean_worker::enrich_bookmark(&pool, &search, uid, id, &opts).await;
        });
    }
    tokio::spawn(crate::archive_runner::rekey_after_vault_move(
        state.clone(),
        keyring,
        user.id,
    ));
    Ok(Json(json!({ "ok": true })).into_response())
}

/// Unlocking needs a signed-in session. API keys never hold vault keys.
fn require_session(session: &VaultSession) -> Result<(), Error> {
    if session.is_session() {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
