use super::assets::{self, Asset, PUBLIC_CACHE};
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::Json;
use linkdean_core::{Error, PublicListView};
use linkdean_db::{self as db};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_public_list))
        .routes(routes!(get_public_preview))
        .routes(routes!(get_public_favicon))
        .routes(routes!(get_public_og))
}

#[utoipa::path(
    get,
    operation_id = "public_get_public_list",
    path = "/public/lists/{slug}",
    tag = "public",
    params(("slug" = String, Path)),
    responses(
        (status = 200, description = "OK", body = PublicListView),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn get_public_list(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<PublicListView>> {
    let list = db::get_public_list_by_slug(&state.pool, &slug).await?;
    Ok(Json(list))
}

#[utoipa::path(
    get,
    operation_id = "public_get_public_preview",
    path = "/public/lists/{slug}/bookmarks/{id}/preview",
    tag = "public",
    params(("slug" = String, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn get_public_preview(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    let bookmark = db::get_public_list_bookmark(&state.pool, &slug, id).await?;
    if !bookmark.has_preview {
        return Err(Error::NotFound.into());
    }
    assets::serve(&state, id, Asset::Preview, PUBLIC_CACHE).await
}

#[utoipa::path(
    get,
    operation_id = "public_get_public_favicon",
    path = "/public/lists/{slug}/bookmarks/{id}/favicon",
    tag = "public",
    params(("slug" = String, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn get_public_favicon(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    db::get_public_list_bookmark(&state.pool, &slug, id).await?;
    assets::serve(&state, id, Asset::Favicon, PUBLIC_CACHE).await
}

#[utoipa::path(
    get,
    operation_id = "public_get_public_og",
    path = "/public/lists/{slug}/bookmarks/{id}/og",
    tag = "public",
    params(("slug" = String, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
)]
async fn get_public_og(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    db::get_public_list_bookmark(&state.pool, &slug, id).await?;
    assets::serve(&state, id, Asset::Og, PUBLIC_CACHE).await
}
