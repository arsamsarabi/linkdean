use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use linkdean_core::{Tag, TeamRole};
use linkdean_db::{self as db};
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
    #[serde(default)]
    pub team_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    operation_id = "tags_list",
    path = "/tags",
    tag = "tags",
    params(ListQuery),
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
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<Tag>>> {
    if let Some(team_id) = q.team_id {
        db::require_team_role(&state.pool, user.id, team_id, TeamRole::Viewer).await?;
        return Ok(Json(db::list_team_tags(&state.pool, team_id).await?));
    }
    Ok(Json(db::list_tags(&state.pool, user.id).await?))
}
