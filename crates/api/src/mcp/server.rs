//! MCP server handler: library read tools plus scoped bookmark writes.

use super::dto::{
    McpArchiveStatusList, McpArchiveText, McpBookmark, McpBookmarkList, McpFolderList, McpOk,
    McpTagList,
};
use super::tools::{
    create_bookmark, delete_bookmark, get_archive_status, get_archive_text, get_bookmark,
    list_folders, list_tags, require_mcp_scope, require_user_state, search_bookmarks,
    update_bookmark,
};
use axum::http::request::Parts;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::handler::server::ServerHandler;
use rmcp::model::{ErrorData, Implementation, ProtocolVersion, ServerCapabilities, ServerConfig};
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct LinkdeanMcp {
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

impl Default for LinkdeanMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkdeanMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchBookmarksParams {
    /// Free-text query. Empty / omitted with folder and/or tag browses the library.
    #[serde(default)]
    pub q: Option<String>,
    /// Folder id; includes subfolders.
    #[serde(default)]
    pub folder: Option<String>,
    /// Tag name filter (case-insensitive).
    #[serde(default)]
    pub tag: Option<String>,
    /// Page size 1..=100 (default 50).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetBookmarkParams {
    /// Bookmark id.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetArchiveStatusParams {
    /// Bookmark id whose archive copies to inspect.
    pub bookmark_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetArchiveTextParams {
    /// Bookmark id whose HTML archive to read as Markdown.
    pub bookmark_id: String,
    /// Archive format; only `html` is supported (default).
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBookmarkParams {
    /// Bookmark URL (required).
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Single folder id (non-vault). Prefer `folders` for several.
    #[serde(default)]
    pub folder: Option<String>,
    /// Folder ids (non-vault). Combined with `folder` when both are set.
    #[serde(default)]
    pub folders: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub is_unread: Option<bool>,
    #[serde(default)]
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateBookmarkParams {
    /// Bookmark id to update.
    pub id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    /// Set to replace notes; omit to leave unchanged. Empty string clears.
    #[serde(default)]
    pub notes: Option<String>,
    /// When set, replaces the bookmark's folders (non-vault only).
    #[serde(default)]
    pub folders: Option<Vec<String>>,
    /// When set, replaces tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub is_unread: Option<bool>,
    #[serde(default)]
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteBookmarkParams {
    /// Bookmark id to soft-delete (trash).
    pub id: String,
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, ErrorData> {
    Uuid::parse_str(raw.trim())
        .map_err(|_| ErrorData::invalid_params(format!("invalid {field} uuid"), None))
}

fn parse_folder_ids(
    folder: Option<&str>,
    folders: Option<&[String]>,
) -> Result<Vec<Uuid>, ErrorData> {
    let mut ids = Vec::new();
    if let Some(s) = folder.map(str::trim).filter(|s| !s.is_empty()) {
        ids.push(parse_uuid(s, "folder")?);
    }
    if let Some(list) = folders {
        for (i, raw) in list.iter().enumerate() {
            let t = raw.trim();
            if t.is_empty() {
                continue;
            }
            let id = parse_uuid(t, &format!("folders[{i}]"))?;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

#[tool_router]
impl LinkdeanMcp {
    #[tool(
        name = "search_bookmarks",
        description = "Search or browse bookmarks. Pass q for text search; empty q with folder and/or tag browses that scope (folder includes subfolders). No vault."
    )]
    async fn search_bookmarks_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<SearchBookmarksParams>,
    ) -> Result<Json<McpBookmarkList>, ErrorData> {
        require_mcp_scope(&parts, "library:read")?;
        let (user, state) = require_user_state(&parts)?;
        let folder = match params.folder.as_deref() {
            None | Some("") => None,
            Some(s) => Some(parse_uuid(s, "folder")?),
        };
        let list =
            search_bookmarks(&state, &user, params.q, folder, params.tag, params.limit).await?;
        Ok(Json(list))
    }

    #[tool(
        name = "get_bookmark",
        description = "Get one bookmark by id (title, url, notes, tags, folders, dates). Vault bookmarks are unavailable."
    )]
    async fn get_bookmark_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<GetBookmarkParams>,
    ) -> Result<Json<McpBookmark>, ErrorData> {
        require_mcp_scope(&parts, "library:read")?;
        let (user, state) = require_user_state(&parts)?;
        let id = parse_uuid(&params.id, "id")?;
        let bookmark = get_bookmark(&state, &user, id).await?;
        Ok(Json(bookmark))
    }

    #[tool(
        name = "list_folders",
        description = "List folders the user can see (owned and shared). Vault folders are excluded."
    )]
    async fn list_folders_tool(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<McpFolderList>, ErrorData> {
        require_mcp_scope(&parts, "library:read")?;
        let (user, state) = require_user_state(&parts)?;
        Ok(Json(list_folders(&state, &user).await?))
    }

    #[tool(
        name = "list_tags",
        description = "List tag names in the user's library."
    )]
    async fn list_tags_tool(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<Json<McpTagList>, ErrorData> {
        require_mcp_scope(&parts, "library:read")?;
        let (user, state) = require_user_state(&parts)?;
        Ok(Json(list_tags(&state, &user).await?))
    }

    #[tool(
        name = "get_archive_status",
        description = "Archive copy status for a bookmark (format, status, bytes, captured_at). Metadata only — no file bodies. Vault bookmarks unavailable."
    )]
    async fn get_archive_status_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<GetArchiveStatusParams>,
    ) -> Result<Json<McpArchiveStatusList>, ErrorData> {
        require_mcp_scope(&parts, "archive:read")?;
        let (user, state) = require_user_state(&parts)?;
        let bookmark_id = parse_uuid(&params.bookmark_id, "bookmark_id")?;
        Ok(Json(get_archive_status(&state, &user, bookmark_id).await?))
    }

    #[tool(
        name = "get_archive_text",
        description = "Reader-mode Markdown from a bookmark's HTML archive copy when one exists. Vault bookmarks unavailable; PDF/file copies are not supported."
    )]
    async fn get_archive_text_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<GetArchiveTextParams>,
    ) -> Result<Json<McpArchiveText>, ErrorData> {
        require_mcp_scope(&parts, "archive:read")?;
        let (user, state) = require_user_state(&parts)?;
        let bookmark_id = parse_uuid(&params.bookmark_id, "bookmark_id")?;
        Ok(Json(
            get_archive_text(&state, &user, bookmark_id, params.format.as_deref()).await?,
        ))
    }

    #[tool(
        name = "create_bookmark",
        description = "Create a bookmark (url required). Optional title, notes, tags, folders. Needs library:write. Vault folders are refused."
    )]
    async fn create_bookmark_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<CreateBookmarkParams>,
    ) -> Result<Json<McpBookmark>, ErrorData> {
        require_mcp_scope(&parts, "library:write")?;
        let (user, state) = require_user_state(&parts)?;
        let folder_ids = parse_folder_ids(params.folder.as_deref(), params.folders.as_deref())?;
        let bookmark = create_bookmark(
            &state,
            &user,
            params.url,
            params.title,
            params.notes,
            folder_ids,
            params.tags.unwrap_or_default(),
            params.is_favorite.unwrap_or(false),
            params.is_unread.unwrap_or(false),
            params.is_pinned.unwrap_or(false),
        )
        .await?;
        Ok(Json(bookmark))
    }

    #[tool(
        name = "update_bookmark",
        description = "Update a bookmark by id. Pass only fields to change; folders/tags replace when set. Needs library:write. Vault bookmarks unavailable."
    )]
    async fn update_bookmark_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<UpdateBookmarkParams>,
    ) -> Result<Json<McpBookmark>, ErrorData> {
        require_mcp_scope(&parts, "library:write")?;
        let (user, state) = require_user_state(&parts)?;
        let id = parse_uuid(&params.id, "id")?;
        let folders = match params.folders {
            None => None,
            Some(list) => Some(parse_folder_ids(None, Some(&list))?),
        };
        let notes = params.notes.map(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });
        let bookmark = update_bookmark(
            &state,
            &user,
            id,
            params.url,
            params.title,
            notes,
            folders,
            params.tags,
            params.is_favorite,
            params.is_unread,
            params.is_pinned,
        )
        .await?;
        Ok(Json(bookmark))
    }

    #[tool(
        name = "delete_bookmark",
        description = "Soft-delete (trash) a bookmark by id. Needs library:write. Vault bookmarks unavailable."
    )]
    async fn delete_bookmark_tool(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<DeleteBookmarkParams>,
    ) -> Result<Json<McpOk>, ErrorData> {
        require_mcp_scope(&parts, "library:write")?;
        let (user, state) = require_user_state(&parts)?;
        let id = parse_uuid(&params.id, "id")?;
        Ok(Json(delete_bookmark(&state, &user, id).await?))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LinkdeanMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("linkdean", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2025_03_26)
            .with_instructions(
                "Linkdean library tools. Search and read bookmarks, folders, tags, archive status, and HTML archive reader text. Create, update, or trash bookmarks when the API key has library:write. Vault is never available."
                    .to_string(),
            )
    }
}
