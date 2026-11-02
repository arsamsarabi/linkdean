use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use linkdean_core::{PurgeTrashRequest, TrashItem, TrashKind};
use linkdean_db::{self as db};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/trash", get(list))
        .route("/trash/purge", post(purge_selected))
        .route("/trash/{kind}/{id}/restore", post(restore))
        .route("/trash/{kind}/{id}", delete(purge_one))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<TrashItem>>> {
    let items = db::list_trash(&state.pool, user.id).await?;
    Ok(Json(items))
}

fn parse_kind(kind: &str) -> Result<TrashKind, linkdean_core::Error> {
    match kind {
        "bookmark" => Ok(TrashKind::Bookmark),
        "category" => Ok(TrashKind::Category),
        "dashboard" => Ok(TrashKind::Dashboard),
        _ => Err(linkdean_core::Error::BadRequest(
            "kind must be bookmark|category|dashboard".into(),
        )),
    }
}

async fn restore(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path((kind, id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let kind = parse_kind(&kind)?;
    match kind {
        TrashKind::Bookmark => {
            db::restore_bookmark(&state.pool, user.id, id).await?;
            let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
            let keyring = state.vault_keyring(&vault_session, user.id).await;
            if let Some(view) = db::bookmark_to_view(&state.pool, raw, &keyring, user.id).await? {
                if !view.is_vault {
                    if let Err(e) = state.search.upsert_bookmark(&view).await {
                        tracing::warn!(bookmark_id = %id, error = %e, "meili upsert after restore failed");
                    }
                }
            }
        }
        TrashKind::Category => db::restore_category(&state.pool, user.id, id).await?,
        TrashKind::Dashboard => db::restore_dashboard(&state.pool, user.id, id).await?,
    }
    Ok(Json(json!({ "ok": true })))
}

async fn purge_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((kind, id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Value>> {
    let kind = parse_kind(&kind)?;
    match kind {
        TrashKind::Bookmark => {
            db::purge_bookmark(&state.pool, user.id, id).await?;
            if let Err(e) = state.search.delete_bookmark(id).await {
                tracing::warn!(bookmark_id = %id, error = %e, "meili delete on purge failed");
            }
            linkdean_worker::purge_preview_assets(std::path::Path::new(&state.cfg.preview_dir), id)
                .await;
        }
        TrashKind::Category => db::purge_category(&state.pool, user.id, id).await?,
        TrashKind::Dashboard => db::purge_dashboard(&state.pool, user.id, id).await?,
    }
    Ok(Json(json!({ "ok": true })))
}

async fn purge_selected(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<PurgeTrashRequest>,
) -> ApiResult<Json<Value>> {
    let mut ok = 0u32;
    let mut failed = 0u32;
    for item in body.items {
        let res = match item.kind {
            TrashKind::Bookmark => {
                let r = db::purge_bookmark(&state.pool, user.id, item.id).await;
                if r.is_ok() {
                    let _ = state.search.delete_bookmark(item.id).await;
                    linkdean_worker::purge_preview_assets(
                        std::path::Path::new(&state.cfg.preview_dir),
                        item.id,
                    )
                    .await;
                }
                r
            }
            TrashKind::Category => db::purge_category(&state.pool, user.id, item.id).await,
            TrashKind::Dashboard => db::purge_dashboard(&state.pool, user.id, item.id).await,
        };
        if res.is_ok() {
            ok += 1;
        } else {
            failed += 1;
        }
    }
    Ok(Json(json!({ "ok": true, "purged": ok, "failed": failed })))
}
