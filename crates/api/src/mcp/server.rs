//! MCP server handler with read-only bookmark tools.

use super::dto::{
    McpArchiveStatusList, McpArchiveText, McpBookmark, McpBookmarkList, McpFolderList, McpTagList,
};
use super::tools::{
    get_archive_status, get_archive_text, get_bookmark, list_folders, list_tags, require_mcp_scope,
    require_user_state, search_bookmarks,
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

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, ErrorData> {
    Uuid::parse_str(raw.trim())
        .map_err(|_| ErrorData::invalid_params(format!("invalid {field} uuid"), None))
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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LinkdeanMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("linkdean", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2025_03_26)
            .with_instructions(
                "Linkdean read-only library tools. Search and read bookmarks, folders, tags, archive status, and HTML archive reader text; no writes, no vault."
                    .to_string(),
            )
    }
}
