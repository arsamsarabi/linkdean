//! `GET /api/agent-skill`: a folder or tag as an Agent Skill (<https://agentskills.io>), for an
//! agent's skills folder. Works with an API key, so a script can refresh it.

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use linkdean_core::agent_skill::build_skill;
use linkdean_core::Error;
use linkdean_db::{self as db, SkillFrom};
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/agent-skill", get(agent_skill))
}

#[derive(Debug, Deserialize)]
struct SkillQuery {
    folder: Option<Uuid>,
    tag: Option<String>,
    /// `zip` (default): a `<name>/` folder. `md`: the `SKILL.md` alone, every link inline.
    format: Option<String>,
}

fn source(q: &SkillQuery) -> Result<SkillFrom, Error> {
    match (q.folder, q.tag.as_deref().map(str::trim)) {
        (Some(id), None) => Ok(SkillFrom::Folder(id)),
        (None, Some(tag)) if !tag.is_empty() => Ok(SkillFrom::Tag(tag.to_string())),
        _ => Err(Error::BadRequest(
            "give either folder or tag, not both".into(),
        )),
    }
}

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
    let found = db::agent_skill_bookmarks(&state.pool, user.id, &from).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn q(folder: Option<Uuid>, tag: Option<&str>) -> SkillQuery {
        SkillQuery {
            folder,
            tag: tag.map(str::to_string),
            format: None,
        }
    }

    #[test]
    fn a_skill_comes_from_one_folder_or_one_tag() {
        let id = Uuid::new_v4();
        assert!(matches!(source(&q(Some(id), None)), Ok(SkillFrom::Folder(f)) if f == id));
        assert!(matches!(source(&q(None, Some(" rust "))), Ok(SkillFrom::Tag(t)) if t == "rust"));
        assert!(source(&q(Some(id), Some("rust"))).is_err());
        assert!(source(&q(None, Some("  "))).is_err());
        assert!(source(&q(None, None)).is_err());
    }
}
