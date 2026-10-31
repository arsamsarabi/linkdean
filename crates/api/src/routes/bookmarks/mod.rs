use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{BookmarkView, CreateBookmark, Error, UpdateBookmark};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bookmarks", get(list).post(create))
        .route("/bookmarks/duplicates", get(duplicates))
        .route("/bookmarks/{id}/preview", get(get_preview))
        .route(
            "/bookmarks/{id}",
            get(get_one).patch(update).delete(delete_one),
        )
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub include_vault: bool,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<BookmarkView>>> {
    let key = state.vault_key(user.id).await;
    let include_vault = q.include_vault && key.is_some();
    let items = db::list_bookmarks(
        &state.pool,
        user.id,
        q.category_id,
        include_vault,
        key.as_ref(),
    )
    .await?;
    Ok(Json(items))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BookmarkView>> {
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    let key = state.vault_key(user.id).await;
    let view = db::bookmark_to_view(&state.pool, raw, key.as_ref(), user.id)
        .await?
        .ok_or(linkdean_core::Error::VaultLocked)?;
    Ok(Json(view))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateBookmark>,
) -> ApiResult<Json<BookmarkView>> {
    let key = state.vault_key(user.id).await;
    let view = db::create_bookmark(&state.pool, user.id, req, key.as_ref()).await?;
    if !view.is_vault {
        if let Err(e) = state.search.upsert_bookmark(&view).await {
            tracing::warn!(bookmark_id = %view.id, error = %e, "meili upsert failed");
        }
        let pool = state.pool.clone();
        let search = state.search.clone();
        let opts = state.enrich_opts();
        let uid = user.id;
        let bid = view.id;
        tokio::spawn(async move {
            linkdean_worker::enrich_bookmark(&pool, &search, uid, bid, &opts).await;
        });
    }
    Ok(Json(view))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBookmark>,
) -> ApiResult<Json<BookmarkView>> {
    let key = state.vault_key(user.id).await;
    let view = db::update_bookmark(&state.pool, user.id, id, req, key.as_ref()).await?;
    if view.is_vault {
        if let Err(e) = state.search.delete_bookmark(id).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili delete failed");
        }
    } else if let Err(e) = state.search.upsert_bookmark(&view).await {
        tracing::warn!(bookmark_id = %view.id, error = %e, "meili upsert failed");
    }
    Ok(Json(view))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    db::delete_bookmark(&state.pool, user.id, id).await?;
    if let Err(e) = state.search.delete_bookmark(id).await {
        tracing::warn!(bookmark_id = %id, error = %e, "meili delete failed");
    }
    let path = state.preview_path(id);
    if let Err(e) = tokio::fs::remove_file(&path).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(?path, error = %e, "preview delete failed");
        }
    }
    Ok(Json(json!({ "ok": true })))
}

async fn get_preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    if !raw.has_preview {
        return Err(Error::NotFound.into());
    }
    let path = state.preview_path(id);
    let bytes = tokio::fs::read(&path).await.map_err(|_| Error::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok((StatusCode::OK, headers, bytes))
}

async fn duplicates(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Value>> {
    let urls = db::find_duplicate_urls(&state.pool, user.id).await?;
    Ok(Json(json!({ "urls": urls })))
}
