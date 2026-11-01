use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use linkdean_core::{ActivityAction, ActivitySubjectType, ListActivityFilter};
use linkdean_db::{self as db};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new().route("/activity", get(list))
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    subject_type: Option<String>,
    action: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let subject_type = match q.subject_type.as_deref() {
        None | Some("") => None,
        Some(s) => Some(ActivitySubjectType::parse(s).map_err(linkdean_core::Error::BadRequest)?),
    };
    let action = match q.action.as_deref() {
        None | Some("") => None,
        Some(s) => Some(ActivityAction::parse(s).map_err(linkdean_core::Error::BadRequest)?),
    };
    let filter = ListActivityFilter {
        subject_type,
        action,
        limit: q.limit.unwrap_or(50),
    };
    let items = db::list_activity(&state.pool, user.id, filter).await?;
    Ok(Json(json!({ "events": items })))
}
