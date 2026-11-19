//! MCP tool handlers: search/read bookmarks, folders, tags, archive status.

use super::dto::{
    McpArchiveStatus, McpArchiveStatusList, McpBookmark, McpBookmarkList, McpFolderList, McpTagList,
};
use crate::state::AppState;
use axum::http::request::Parts;
use linkdean_core::{Error, User, VaultKeyring};
use linkdean_db as db;
use rmcp::model::ErrorData;
use uuid::Uuid;

pub fn mcp_err(err: Error) -> ErrorData {
    // Agents only get actionable client messages; hide internal/auth detail.
    let message = match &err {
        Error::NotFound => "not found".to_string(),
        Error::BadRequest(msg) => msg.clone(),
        _ => "request failed".to_string(),
    };
    ErrorData::invalid_params(message, None)
}

/// Agent Skills stance: Only-me bookmarks are owner-visible only, even when a
/// share would otherwise grant `bookmark_can_read`.
fn others_only_me(is_private: bool, owner_user_id: Uuid, viewer_id: Uuid) -> bool {
    is_private && owner_user_id != viewer_id
}

pub fn require_user_state(parts: &Parts) -> Result<(User, AppState), ErrorData> {
    let user = parts
        .extensions
        .get::<User>()
        .cloned()
        .ok_or_else(|| ErrorData::internal_error("missing authenticated user", None))?;
    let state = parts
        .extensions
        .get::<AppState>()
        .cloned()
        .ok_or_else(|| ErrorData::internal_error("missing app state", None))?;
    Ok((user, state))
}

fn clamp_search_limit(limit: Option<u32>) -> usize {
    match limit {
        None => 50,
        Some(n) => (n as usize).clamp(1, 100),
    }
}

fn tag_matches(tags: &[String], needle: &str) -> bool {
    let needle = needle.trim();
    if needle.is_empty() {
        return true;
    }
    tags.iter().any(|t| t.eq_ignore_ascii_case(needle))
}

/// Empty `q` → DB browse (folder and/or tag required). Non-empty → Meili then optional filters.
pub async fn search_bookmarks(
    state: &AppState,
    user: &User,
    q: Option<String>,
    folder: Option<Uuid>,
    tag: Option<String>,
    limit: Option<u32>,
) -> Result<McpBookmarkList, ErrorData> {
    let q_trim = q.as_deref().unwrap_or("").trim();
    let tag_trim = tag.as_deref().map(str::trim).filter(|s| !s.is_empty());

    if q_trim.is_empty() {
        if folder.is_none() && tag_trim.is_none() {
            return Err(ErrorData::invalid_params(
                "give q, folder, and/or tag",
                None,
            ));
        }
        let limit_i64 = match limit {
            None => 50,
            Some(n) if n < 1 => 50,
            Some(n) => i64::from(n).min(100),
        };
        let views = db::mcp_browse_bookmarks(&state.pool, user.id, folder, tag_trim, limit_i64)
            .await
            .map_err(mcp_err)?;
        return Ok(McpBookmarkList {
            bookmarks: views
                .iter()
                .map(|v| McpBookmark::from_view(v, user.id))
                .collect(),
        });
    }

    let limit = clamp_search_limit(limit);
    // Same Meili visibility set as REST search with no vault unlock.
    let readable = db::list_categories(&state.pool, user.id, true, &[], false)
        .await
        .map_err(mcp_err)?;
    let readable_ids: Vec<Uuid> = readable.into_iter().map(|c| c.id).collect();

    let docs = match state
        .search
        .search(user.id, &readable_ids, q_trim, limit)
        .await
    {
        Ok(docs) => docs,
        Err(e) => {
            tracing::warn!("meili search failed: {e}");
            return Ok(McpBookmarkList {
                bookmarks: Vec::new(),
            });
        }
    };

    let ids: Vec<Uuid> = docs
        .iter()
        .filter_map(|d| Uuid::parse_str(&d.id).ok())
        .collect();
    let live = db::live_plain_bookmark_ids(&state.pool, &ids)
        .await
        .map_err(mcp_err)?;
    let views = db::plain_bookmark_views_by_ids(&state.pool, user.id, &ids)
        .await
        .map_err(mcp_err)?;

    let folder_scope = if let Some(folder_id) = folder {
        Some(
            db::mcp_folder_descendant_ids(&state.pool, user.id, folder_id)
                .await
                .map_err(mcp_err)?,
        )
    } else {
        None
    };

    let mut bookmarks = Vec::new();
    for d in docs {
        let Ok(id) = Uuid::parse_str(&d.id) else {
            continue;
        };
        if !live.contains(&id) {
            continue;
        }
        if !db::bookmark_can_read(&state.pool, user.id, id)
            .await
            .unwrap_or(false)
        {
            continue;
        }
        let Some(view) = views.get(&id) else {
            continue;
        };
        // Match empty-q browse / Agent Skills: hide others' Only-me from sharees.
        if others_only_me(view.is_private, view.owner_user_id, user.id) {
            continue;
        }
        if let Some(ref scope) = folder_scope {
            let in_folder = view.categories.iter().any(|c| scope.contains(&c.id))
                || view.category_id.is_some_and(|cid| scope.contains(&cid));
            if !in_folder {
                continue;
            }
        }
        if let Some(tag) = tag_trim {
            if !tag_matches(&view.tags, tag) {
                continue;
            }
        }
        bookmarks.push(McpBookmark::from_view(view, user.id));
        if bookmarks.len() >= limit {
            break;
        }
    }

    Ok(McpBookmarkList { bookmarks })
}

pub async fn get_bookmark(
    state: &AppState,
    user: &User,
    id: Uuid,
) -> Result<McpBookmark, ErrorData> {
    let raw = db::get_bookmark_raw(&state.pool, user.id, id)
        .await
        .map_err(mcp_err)?;
    if raw.is_vault {
        return Err(ErrorData::invalid_params(
            "vault not available via MCP",
            None,
        ));
    }
    if others_only_me(raw.is_private, raw.owner_user_id, user.id) {
        return Err(mcp_err(Error::NotFound));
    }
    let view = db::bookmark_to_view(&state.pool, raw, &VaultKeyring::default(), user.id)
        .await
        .map_err(mcp_err)?
        .ok_or_else(|| ErrorData::invalid_params("not found", None))?;
    Ok(McpBookmark::from_view(&view, user.id))
}

/// Visible folders including shared; vault folders omitted.
pub async fn list_folders(state: &AppState, user: &User) -> Result<McpFolderList, ErrorData> {
    let cats = db::list_categories(&state.pool, user.id, true, &[], false)
        .await
        .map_err(mcp_err)?;
    let visible: Vec<_> = cats.into_iter().filter(|c| !c.is_vault).collect();
    Ok(McpFolderList::from_categories(&visible))
}

pub async fn list_tags(state: &AppState, user: &User) -> Result<McpTagList, ErrorData> {
    let tags = db::list_tags(&state.pool, user.id).await.map_err(mcp_err)?;
    Ok(McpTagList::from_tags(&tags))
}

/// Archive format/status metadata only — no file bodies.
pub async fn get_archive_status(
    state: &AppState,
    user: &User,
    bookmark_id: Uuid,
) -> Result<McpArchiveStatusList, ErrorData> {
    let raw = db::get_bookmark_raw(&state.pool, user.id, bookmark_id)
        .await
        .map_err(mcp_err)?;
    if raw.is_vault {
        return Err(ErrorData::invalid_params(
            "vault not available via MCP",
            None,
        ));
    }
    if others_only_me(raw.is_private, raw.owner_user_id, user.id) {
        return Err(mcp_err(Error::NotFound));
    }
    let views = db::list_archives(&state.pool, bookmark_id)
        .await
        .map_err(mcp_err)?;
    Ok(McpArchiveStatusList {
        archives: views.iter().map(McpArchiveStatus::from_view).collect(),
    })
}
