use super::assets::{self, Asset, PRIVATE_CACHE};
use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::vault_keys::VaultSession;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use linkdean_core::preview_image::{self, process_bookmark_preview};
use linkdean_core::VaultKeyring;
use linkdean_core::{BookmarkView, CreateBookmark, Error, UpdateBookmark};
use linkdean_db::{self as db, BookmarkPageFilter, BookmarkSort, DEFAULT_PAGE_LIMIT};
use serde::{Deserialize, Serialize};
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
        .route("/bookmarks/ids", get(list_ids))
        .route("/bookmarks/page-details", post(page_details))
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
    pub folder: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
    /// Comma-separated or single tag name(s).
    pub tag: Option<String>,
    /// Comma-separated or single flag name(s).
    pub flag: Option<String>,
    /// RFC3339 timestamp lower bound on `created_at`.
    pub since: Option<String>,
    #[serde(default)]
    pub include_vault: bool,
    /// Alias: `vault=1` also sets vault-only filter.
    pub vault: Option<String>,
    #[serde(default)]
    pub vault_only: bool,
}

#[derive(Serialize)]
pub struct BookmarkListResponse {
    pub items: Vec<BookmarkView>,
    pub next_cursor: Option<String>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct BookmarkIdsResponse {
    pub ids: Vec<Uuid>,
}

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

fn page_filter(
    q: &ListQuery,
    include_vault: bool,
    include_deleted: bool,
) -> Result<BookmarkPageFilter, Error> {
    let folder = q
        .folder
        .clone()
        .or_else(|| q.category_id.map(|id| id.to_string()));
    let since = match q.since.as_deref() {
        None | Some("") => None,
        Some(s) => Some(
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| Error::BadRequest("invalid since".into()))?,
        ),
    };
    let vault_only = q.vault_only || matches!(q.vault.as_deref(), Some("1") | Some("true"));
    Ok(BookmarkPageFilter {
        folder,
        tags: split_csv(q.tag.as_deref()),
        flags: split_csv(q.flag.as_deref()),
        since,
        vault_only,
        include_vault,
        include_deleted,
        sort: BookmarkSort::parse(q.sort.as_deref().unwrap_or("")),
        limit: q.limit.unwrap_or(DEFAULT_PAGE_LIMIT),
        cursor: q.cursor.clone(),
    })
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<BookmarkListResponse>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let include_vault = q.include_vault && !keyring.is_empty();
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let filter = page_filter(&q, include_vault, settings.show_deleted_in_library)?;
    let page = db::list_bookmarks_page(&state.pool, user.id, filter, &keyring).await?;
    Ok(Json(BookmarkListResponse {
        items: page.items,
        next_cursor: page.next_cursor,
        total: page.total,
    }))
}

async fn list_ids(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<BookmarkIdsResponse>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let include_vault = q.include_vault && !keyring.is_empty();
    let settings = db::get_user_settings(&state.pool, user.id).await?;
    let filter = page_filter(&q, include_vault, settings.show_deleted_in_library)?;
    let ids = db::list_bookmark_ids_matching(&state.pool, user.id, filter, &keyring).await?;
    Ok(Json(BookmarkIdsResponse { ids }))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BookmarkView>> {
    let raw = db::get_bookmark_raw(&state.pool, user.id, id).await?;
    let keyring = state.vault_keyring(&vault_session, user.id).await;
    let view = db::bookmark_to_view(&state.pool, raw, &keyring, user.id)
        .await?
        .ok_or(linkdean_core::Error::VaultLocked)?;
    Ok(Json(view))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
    Json(req): Json<CreateBookmark>,
) -> ApiResult<Json<BookmarkView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
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
    vault_session: VaultSession,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBookmark>,
) -> ApiResult<Json<BookmarkView>> {
    let keyring = state.vault_keyring(&vault_session, user.id).await;
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
) -> ApiResult<Response> {
    let raw = db::get_bookmark_for_assets(&state.pool, user.id, id).await?;
    if !raw.has_preview {
        return Err(Error::NotFound.into());
    }
    assets::serve(&state, id, Asset::Preview, PRIVATE_CACHE).await
}

async fn upload_preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    vault_session: VaultSession,
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
    let limits = crate::quotas::limits_for(&state.pool).await?;
    crate::quotas::refuse_upload(
        &state,
        user.id,
        &limits,
        raw_bytes.len(),
        crate::quotas::HARD_PREVIEW_BODY,
    )
    .await?;
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
    let keyring = state.vault_keyring(&vault_session, user.id).await;
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
    vault_session: VaultSession,
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
    let keyring = state.vault_keyring(&vault_session, user.id).await;
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
) -> ApiResult<Response> {
    db::get_bookmark_for_assets(&state.pool, user.id, id).await?;
    assets::serve(&state, id, Asset::Favicon, PRIVATE_CACHE).await
}

async fn get_og(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    db::get_bookmark_for_assets(&state.pool, user.id, id).await?;
    assets::serve(&state, id, Asset::Og, PRIVATE_CACHE).await
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

#[derive(Deserialize)]
pub struct PageDetailsRequest {
    pub url: String,
}

/// Fetches a page's title, description and keyword tags so the bookmark form can fill in
/// what the person has not typed yet. Nothing is saved. Counts as one enrich action and is
/// rate limited per user. The SPA never calls it for a bookmark headed into a vault.
async fn page_details(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<PageDetailsRequest>,
) -> ApiResult<Json<Value>> {
    let url = req.url.trim();
    db::require_web_url(url)?;
    let rate = crate::rate_limit::page_details_limit(user.id);
    state.limiter.check(&rate)?;
    state.limiter.record(&rate);
    let limits = crate::quotas::limits_for(&state.pool).await?;
    db::consume_enrich_action(&state.pool, user.id, &limits).await?;

    use linkdean_worker::PageDetailsError;
    let details = linkdean_worker::fetch_page_details(url)
        .await
        .map_err(|e| {
            Error::BadRequest(match e {
                PageDetailsError::NotPublic => {
                    "Only public web addresses can be fetched.".to_string()
                }
                PageDetailsError::Refused(code) => {
                    format!(
                        "The site refused the request (HTTP {code}). Fill in the details yourself."
                    )
                }
                PageDetailsError::Unreachable => {
                    "Couldn't reach that page. Check the address, or fill in the details yourself."
                        .to_string()
                }
            })
        })?;
    Ok(Json(json!({
        "title": details.title,
        "description": details.description,
        "tags": details.tags,
    })))
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
