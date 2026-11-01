use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use linkdean_core::{
    ChangeGlobalPassphraseRequest, CreateVaultRequest, DeleteVaultRequest, Error,
    MoveToVaultRequest, RecoverVaultRequest, RenameVaultRequest, Vault, VaultKeyring,
    VaultMoveOutcome, VaultSetupRequest, VaultUnlockRequest, VaultView,
};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/vault/setup", post(setup))
        .route("/vault/unlock", post(unlock))
        .route("/vault/lock", post(lock))
        .route("/vault/status", get(status))
        .route("/vaults", get(list).post(create))
        .route("/vaults/lock-all", post(lock_all))
        .route("/vaults/move", post(move_items))
        .route(
            "/vaults/global",
            get(global_status)
                .post(global_enable)
                .put(global_change)
                .delete(global_disable),
        )
        .route("/vaults/global/unlock", post(global_unlock))
        .route("/vaults/{id}", patch(rename).delete(delete_one))
        .route("/vaults/{id}/unlock", post(unlock_one))
        .route("/vaults/{id}/lock", post(lock_one))
        .route("/vaults/{id}/recover", post(recover))
        .route("/vaults/{id}/recovery-key", post(new_recovery_key))
        .route(
            "/vaults/{id}/global",
            post(global_join).delete(global_leave),
        )
}

async fn setup(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultSetupRequest>,
) -> ApiResult<Json<Value>> {
    if req.passphrase.len() < 8 {
        return Err(Error::BadRequest("vault passphrase min 8 chars".into()).into());
    }
    db::setup_vault(&state.pool, user.id, &req.passphrase).await?;
    let vault = db::get_vault(&state.pool, user.id).await?;
    let key = db::unlock_vault_key(&state.pool, user.id, &req.passphrase).await?;
    state.set_vault_key(user.id, vault.id, key).await;
    Ok(Json(json!({ "ok": true, "unlocked": true })))
}

async fn unlock(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let vault = db::get_vault(&state.pool, user.id).await?;
    let key = db::unlock_vault_key(&state.pool, user.id, &req.passphrase).await?;
    state.set_vault_key(user.id, vault.id, key).await;
    Ok(Json(json!({ "ok": true, "unlocked": true })))
}

async fn lock(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    state.clear_vault_keys(user.id).await;
    Ok(Json(json!({ "ok": true, "unlocked": false })))
}

async fn status(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({
        "configured": db::user_has_vault(&state.pool, user.id).await?,
        "unlocked": !state.vault_keyring(user.id).await.is_empty(),
    })))
}

fn check_passphrase(passphrase: &str) -> Result<(), crate::error::ApiError> {
    if passphrase.len() < 8 {
        return Err(Error::BadRequest("vault passphrase min 8 chars".into()).into());
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

async fn list(State(state): State<AppState>, AuthUser(user): AuthUser) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(user.id).await;
    let mut vaults = Vec::new();
    for vault in db::list_vaults(&state.pool, user.id).await? {
        vaults.push(to_view(&state, &keyring, vault).await?);
    }
    Ok(Json(json!({ "vaults": vaults })))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateVaultRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.passphrase)?;
    let (vault, key, recovery_key) =
        db::create_vault(&state.pool, user.id, &req.name, &req.passphrase).await?;
    state.set_vault_key(user.id, vault.id, key).await;
    let keyring = state.vault_keyring(user.id).await;
    let view = to_view(&state, &keyring, vault).await?;
    Ok(Json(json!({ "vault": view, "recovery_key": recovery_key })))
}

async fn rename(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameVaultRequest>,
) -> ApiResult<Json<Value>> {
    let vault = db::rename_vault(&state.pool, user.id, id, &req.name).await?;
    let keyring = state.vault_keyring(user.id).await;
    let view = to_view(&state, &keyring, vault).await?;
    Ok(Json(json!({ "vault": view })))
}

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

async fn unlock_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let key = db::unlock_vault(&state.pool, user.id, id, &req.passphrase).await?;
    state.set_vault_key(user.id, id, key).await;
    Ok(Json(json!({ "ok": true })))
}

async fn lock_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::get_user_vault(&state.pool, user.id, id).await?;
    state.clear_vault_key(user.id, id).await;
    Ok(Json(json!({ "ok": true })))
}

async fn lock_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    state.clear_vault_keys(user.id).await;
    Ok(Json(json!({ "ok": true })))
}

async fn recover(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RecoverVaultRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.new_passphrase)?;
    let key = db::recover_vault(
        &state.pool,
        user.id,
        id,
        &req.recovery_key,
        &req.new_passphrase,
    )
    .await?;
    state.set_vault_key(user.id, id, key).await;
    Ok(Json(json!({ "ok": true })))
}

async fn new_recovery_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(user.id).await;
    let key = keyring.get(id).ok_or(Error::VaultLocked)?;
    let recovery_key = db::replace_recovery_key(&state.pool, user.id, id, key).await?;
    Ok(Json(json!({ "recovery_key": recovery_key })))
}

async fn global_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let enabled = db::has_global_passphrase(&state.pool, user.id).await?;
    Ok(Json(json!({ "enabled": enabled })))
}

async fn global_enable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    check_passphrase(&req.passphrase)?;
    db::enable_global_passphrase(&state.pool, user.id, &req.passphrase).await?;
    Ok(Json(json!({ "ok": true })))
}

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

async fn global_disable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    db::disable_global_passphrase(&state.pool, user.id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn global_unlock(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let opened = db::unlock_with_global(&state.pool, user.id, &req.passphrase).await?;
    let mut ids = Vec::with_capacity(opened.len());
    for (id, key) in opened {
        state.set_vault_key(user.id, id, key).await;
        ids.push(id);
    }
    Ok(Json(json!({ "unlocked": ids })))
}

async fn global_join(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<VaultUnlockRequest>,
) -> ApiResult<Json<Value>> {
    let keyring = state.vault_keyring(user.id).await;
    let key = keyring.get(id).ok_or(Error::VaultLocked)?;
    db::join_global(&state.pool, user.id, id, key, &req.passphrase).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn global_leave(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::leave_global(&state.pool, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn move_items(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<MoveToVaultRequest>,
) -> ApiResult<Response> {
    let keyring = state.vault_keyring(user.id).await;
    let outcome = db::move_to_vault(&state.pool, user.id, req, &keyring).await?;
    let (left_search, joined_search) = match outcome {
        VaultMoveOutcome::Blocked(blockers) => {
            let body = json!({ "error": "This move is blocked.", "blockers": blockers });
            return Ok((StatusCode::CONFLICT, Json(body)).into_response());
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
    Ok(Json(json!({ "ok": true })).into_response())
}
