use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use linkdean_core::Tag;
use linkdean_db::{self as db};
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list))
}

#[utoipa::path(
    get,
    operation_id = "tags_list",
    path = "/tags",
    tag = "tags",
    responses(
        (status = 200, description = "OK", body = Vec<Tag>),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<Vec<Tag>>> {
    Ok(Json(db::list_tags(&state.pool, user.id).await?))
}
