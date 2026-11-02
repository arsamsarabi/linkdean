use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use linkdean_core::preview_image::{self, process_bookmark_preview};
use linkdean_core::VaultKeyring;
use linkdean_core::{BookmarkView, CreateBookmark, Error, UpdateBookmark};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// The image limit in `process_bookmark_preview`, plus multipart framing.
const MAX_PREVIEW_BODY: usize = 9 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bookmarks", get(list).post(create))
        .route("/bookmarks/duplicates", get(duplicates))
        // Literal paths must stay registered as their own routes (not under `{id}`) so
        // axum never tries to parse names like `broken` as a Uuid → bare 400.
        .route("/bookmarks/broken", get(broken))
        .route("/bookmarks/broken-links", get(broken))
        .route("/bookmarks/{id}/check-link", post(check_link))
        .route("/bookmarks/{id}/save", post(save_copy))
        .route(
            "/bookmarks/{id}/preview",
            get(get_preview)
                .post(upload_preview)
                .delete(delete_preview)
                .layer(DefaultBodyLimit::max(MAX_PREVIEW_BODY)),
        )
        .route("/bookmarks/{id}/favicon", get(get_favicon))
        .route("/bookmarks/{id}/og", get(get_og))
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
    let keyring = state.vault_keyring(user.id).await;
    let include_vault = q.include_vault && !keyring.is_empty();
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let items = db::list_bookmarks(
        &state.pool,
        user.id,
        q.category_id,
        include_vault,
        &keyring,
        settings.show_deleted_in_library,
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
    let keyring = state.vault_keyring(user.id).await;
    let view = db::bookmark_to_view(&state.pool, raw, &keyring, user.id)
        .await?
        .ok_or(linkdean_core::Error::VaultLocked)?;
    Ok(Json(view))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateBookmark>,
) -> ApiResult<Json<BookmarkView>> {
    let keyring = state.vault_keyring(user.id).await;
    let view = db::create_bookmark(&state.pool, user.id, req, &keyring).await?;
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
    let keyring = state.vault_keyring(user.id).await;
    let view = db::update_bookmark(&state.pool, user.id, id, req, &keyring).await?;
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
    // Soft-delete — keep preview on disk until permanent purge.
    db::delete_bookmark_by_mode(&state.pool, user.id, id).await?;
    if let Err(e) = state.search.delete_bookmark(id).await {
        tracing::warn!(bookmark_id = %id, error = %e, "meili delete failed");
    }
    Ok(Json(json!({ "ok": true })))
}

async fn get_preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let raw = db::get_bookmark_for_assets(&state.pool, user.id, id).await?;
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
    // Remote content on our origin: it may be an image, but it never gets to run.
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("sandbox; default-src 'none'; style-src 'unsafe-inline'"),
    );
    Ok((StatusCode::OK, headers, bytes))
}

async fn upload_preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> ApiResult<Json<BookmarkView>> {
    if !db::bookmark_can_write(&state.pool, user.id, id).await? {
        return Err(Error::Forbidden.into());
    }
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    if raw.is_vault {
        return Err(Error::BadRequest("vault bookmarks cannot have preview images".into()).into());
    }

    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        let ct = field.content_type().map(|s| s.to_string());
        if name != "preview" && name != "image" && name != "file" {
            continue;
        }
        if !preview_image::accept_content_type(ct.as_deref()) {
            return Err(Error::BadRequest("unsupported content type".into()).into());
        }
        let data = field
            .bytes()
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        bytes = Some(data.to_vec());
        break;
    }
    let raw_bytes = bytes.ok_or_else(|| Error::BadRequest("missing preview file field".into()))?;
    let jpeg = tokio::task::spawn_blocking(move || process_bookmark_preview(&raw_bytes))
        .await
        .map_err(|e| Error::msg(e.to_string()))??;

    let path = state.preview_path(id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
    }
    tokio::fs::write(&path, &jpeg)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    db::set_bookmark_has_preview(&state.pool, id, true).await?;

    let updated = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    let keyring = state.vault_keyring(user.id).await;
    let view = db::bookmark_to_view(&state.pool, updated, &keyring, user.id)
        .await?
        .ok_or(Error::NotFound)?;
    if !view.is_vault {
        if let Err(e) = state.search.upsert_bookmark(&view).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili upsert failed");
        }
    }
    Ok(Json(view))
}

async fn delete_preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BookmarkView>> {
    if !db::bookmark_can_write(&state.pool, user.id, id).await? {
        return Err(Error::Forbidden.into());
    }
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    if raw.is_vault {
        return Err(Error::BadRequest("vault bookmarks cannot have preview images".into()).into());
    }

    let path = state.preview_path(id);
    if let Err(e) = tokio::fs::remove_file(&path).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(?path, error = %e, "preview delete failed");
        }
    }
    db::set_bookmark_has_preview(&state.pool, id, false).await?;

    let updated = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    let keyring = state.vault_keyring(user.id).await;
    let view = db::bookmark_to_view(&state.pool, updated, &keyring, user.id)
        .await?
        .ok_or(Error::NotFound)?;
    if !view.is_vault {
        if let Err(e) = state.search.upsert_bookmark(&view).await {
            tracing::warn!(bookmark_id = %id, error = %e, "meili upsert failed");
        }
    }
    Ok(Json(view))
}

async fn get_favicon(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let _raw = db::get_bookmark_for_assets(&state.pool, user.id, id).await?;
    serve_cached_asset(
        state.favicon_path(id),
        state.favicon_ct_path(id),
        "image/png",
    )
    .await
}

async fn get_og(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let _raw = db::get_bookmark_for_assets(&state.pool, user.id, id).await?;
    serve_cached_asset(state.og_path(id), state.og_ct_path(id), "image/jpeg").await
}

async fn serve_cached_asset(
    path: std::path::PathBuf,
    ct_path: std::path::PathBuf,
    default_ct: &'static str,
) -> ApiResult<impl IntoResponse> {
    let bytes = tokio::fs::read(&path).await.map_err(|_| Error::NotFound)?;
    let ct = match tokio::fs::read_to_string(&ct_path).await {
        Ok(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                default_ct.to_string()
            } else {
                trimmed
            }
        }
        Err(_) => default_ct.to_string(),
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&ct).unwrap_or_else(|_| HeaderValue::from_static(default_ct)),
    );
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
    let groups = db::find_duplicate_groups(&state.pool, user.id).await?;
    let urls: Vec<String> = groups.iter().map(|(u, _)| u.clone()).collect();
    let payload: Vec<Value> = groups
        .into_iter()
        .map(|(url, bookmarks)| {
            json!({
                "url": url,
                "bookmarks": bookmarks,
            })
        })
        .collect();
    Ok(Json(json!({ "urls": urls, "groups": payload })))
}

async fn broken(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<BookmarkView>>> {
    let items = db::list_broken_bookmarks(&state.pool, user.id).await?;
    Ok(Json(items))
}

async fn check_link(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BookmarkView>> {
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    if raw.owner_user_id != user.id {
        return Err(Error::Forbidden.into());
    }
    if raw.is_vault {
        return Err(Error::BadRequest("vault bookmarks are not link-checked".into()).into());
    }
    if raw.url.trim().is_empty() {
        return Err(Error::BadRequest("bookmark has no url".into()).into());
    }
    linkdean_worker::check_bookmark_link(&state.pool, id, &raw.url).await;
    let updated = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    let view = db::bookmark_to_view(&state.pool, updated, &VaultKeyring::default(), user.id)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(Json(view))
}

async fn save_copy(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BookmarkView>> {
    let view = db::save_bookmark_copy(&state.pool, user.id, id).await?;
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
