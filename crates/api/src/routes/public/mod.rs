use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::PublicListView;
use linkdean_db::{self as db};

pub fn router() -> Router<AppState> {
    Router::new().route("/public/lists/{slug}", get(get_public_list))
}

async fn get_public_list(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<PublicListView>> {
    let list = db::get_public_list_by_slug(&state.pool, &slug).await?;
    Ok(Json(list))
}
