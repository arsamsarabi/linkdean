//! Visibility-safe bookmark payloads for MCP tools (never vault ciphertext).

use chrono::{DateTime, Utc};
use linkdean_core::BookmarkView;
use schemars::JsonSchema;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpFolderRef {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpBookmark {
    pub id: String,
    pub title: String,
    pub url: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub folders: Vec<McpFolderRef>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpBookmarkList {
    pub bookmarks: Vec<McpBookmark>,
}

impl McpBookmark {
    /// Map a REST/db view for MCP: drop vault fields; hide others' notes (Agent Skills stance).
    pub fn from_view(view: &BookmarkView, viewer_id: Uuid) -> Self {
        let notes = if view.owner_user_id == viewer_id {
            view.notes.clone()
        } else {
            None
        };
        Self {
            id: view.id.to_string(),
            title: view.title.clone(),
            url: view.url.clone(),
            notes,
            tags: view.tags.clone(),
            folders: view
                .categories
                .iter()
                .map(|c| McpFolderRef {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    path: c.path.clone(),
                })
                .collect(),
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}
