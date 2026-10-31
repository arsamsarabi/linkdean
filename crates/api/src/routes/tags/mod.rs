use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::Tag;
use linkdean_db::{self as db};

pub fn router() -> Router<AppState> {
    Router::new().route("/tags", get(list))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<Tag>>> {
    Ok(Json(db::list_tags(&state.pool, user.id).await?))
}
