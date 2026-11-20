//! `GET /api/agent-skill`: a folder, tag, or saved search as an Agent Skill
//! (<https://agentskills.io>), for an agent's skills folder. Works with an API key, so a script
//! can refresh it.

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use linkdean_core::agent_skill::{build_skill, MAX_SKILL_BOOKMARKS};
use linkdean_core::Error;
use linkdean_db::{self as db, SkillFrom};
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(agent_skill))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct SkillQuery {
    folder: Option<Uuid>,
    tag: Option<String>,
    /// Saved search id (live filter matches; vault always excluded).
    search: Option<Uuid>,
    /// `zip` (default): a `<name>/` folder. `md`: the `SKILL.md` alone, every link inline.
    format: Option<String>,
}

fn source(q: &SkillQuery) -> Result<SkillFrom, Error> {
    let folder = q.folder;
    let tag = q.tag.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let search = q.search;
    match (folder, tag, search) {
        (Some(id), None, None) => Ok(SkillFrom::Folder(id)),
        (None, Some(tag), None) => Ok(SkillFrom::Tag(tag.to_string())),
        (None, None, Some(id)) => Ok(SkillFrom::SavedSearch(id)),
        _ => Err(Error::BadRequest(
            "give exactly one of folder, tag, or search".into(),
        )),
    }
}

#[utoipa::path(
    get,
    operation_id = "agent_skills_agent_skill",
    path = "/agent-skill",
    tag = "agent-skill",
    params(SkillQuery),
    responses(
        (status = 200, description = "OK"),
        (status = "4XX", description = "Client error", body = crate::error::ErrorBody),
        (status = "5XX", description = "Server error", body = crate::error::ErrorBody),
    ),
    security(("api_key" = []), ("session_cookie" = [])),
)]
async fn agent_skill(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<SkillQuery>,
) -> ApiResult<Response> {
    let from = source(&q)?;
    let zip = match q.format.as_deref().unwrap_or("zip") {
        "zip" => true,
        "md" | "markdown" => false,
        _ => return Err(Error::BadRequest("format must be zip|md".into()).into()),
    };

    let ranked = match &from {
        SkillFrom::SavedSearch(id) => {
            let view = db::get_saved_search(&state.pool, user.id, *id).await?;
            match view
                .filters
                .q
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(query) => Some(meili_ranked_ids(&state, user.id, query).await?),
                None => None,
            }
        }
        _ => None,
    };

    let found = db::agent_skill_bookmarks_with_rank(&state.pool, user.id, &from, ranked).await?;
    let skill = build_skill(
        &found.source,
        &found.links,
        found.truncated,
        Utc::now(),
        zip,
    );
    let response = if zip {
        (
            [
                (CONTENT_TYPE, "application/zip".to_string()),
                (
                    CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}.zip\"", skill.name),
                ),
                (CACHE_CONTROL, "no-store".to_string()),
            ],
            skill.to_zip(),
        )
            .into_response()
    } else {
        let md = skill
            .files
            .into_iter()
            .next()
            .map(|(_, body)| body)
            .unwrap_or_default();
        (
            [
                (CONTENT_TYPE, "text/markdown; charset=utf-8".to_string()),
                (
                    CONTENT_DISPOSITION,
                    "attachment; filename=\"SKILL.md\"".to_string(),
                ),
                (CACHE_CONTROL, "no-store".to_string()),
            ],
            md,
        )
            .into_response()
    };
    Ok(response)
}

async fn meili_ranked_ids(
    state: &AppState,
    user_id: Uuid,
    query: &str,
) -> Result<Vec<Uuid>, Error> {
    let readable = db::list_categories(&state.pool, user_id, true, &[], false)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let readable_ids: Vec<Uuid> = readable.into_iter().map(|c| c.id).collect();
    let limit = MAX_SKILL_BOOKMARKS + 1;
    let docs = state
        .search
        .search(user_id, &readable_ids, query, limit)
        .await
        .map_err(|e| Error::msg(format!("search failed: {e}")))?;
    Ok(docs
        .into_iter()
        .filter_map(|d| Uuid::parse_str(&d.id).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(folder: Option<Uuid>, tag: Option<&str>, search: Option<Uuid>) -> SkillQuery {
        SkillQuery {
            folder,
            tag: tag.map(str::to_string),
            search,
            format: None,
        }
    }

    #[test]
    fn a_skill_comes_from_one_folder_tag_or_saved_search() {
        let id = Uuid::new_v4();
        let search = Uuid::new_v4();
        assert!(matches!(source(&q(Some(id), None, None)), Ok(SkillFrom::Folder(f)) if f == id));
        assert!(
            matches!(source(&q(None, Some(" rust "), None)), Ok(SkillFrom::Tag(t)) if t == "rust")
        );
        assert!(
            matches!(source(&q(None, None, Some(search))), Ok(SkillFrom::SavedSearch(s)) if s == search)
        );
        assert!(source(&q(Some(id), Some("rust"), None)).is_err());
        assert!(source(&q(Some(id), None, Some(search))).is_err());
        assert!(source(&q(None, Some("rust"), Some(search))).is_err());
        assert!(source(&q(None, Some("  "), None)).is_err());
        assert!(source(&q(None, None, None)).is_err());
    }
}
