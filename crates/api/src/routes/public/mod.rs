use super::assets::{self, Asset, PUBLIC_CACHE};
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{Error, PublicListView};
use linkdean_db::{self as db};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/public/lists/{slug}", get(get_public_list))
        .route(
            "/public/lists/{slug}/bookmarks/{id}/preview",
            get(get_public_preview),
        )
        .route(
            "/public/lists/{slug}/bookmarks/{id}/favicon",
            get(get_public_favicon),
        )
        .route("/public/lists/{slug}/bookmarks/{id}/og", get(get_public_og))
}

async fn get_public_list(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<PublicListView>> {
    let list = db::get_public_list_by_slug(&state.pool, &slug).await?;
    Ok(Json(list))
}

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

async fn get_public_favicon(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    db::get_public_list_bookmark(&state.pool, &slug, id).await?;
    assets::serve(&state, id, Asset::Favicon, PUBLIC_CACHE).await
}

async fn get_public_og(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, Uuid)>,
) -> ApiResult<Response> {
    db::get_public_list_bookmark(&state.pool, &slug, id).await?;
    assets::serve(&state, id, Asset::Og, PUBLIC_CACHE).await
}
