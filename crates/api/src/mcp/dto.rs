//! Visibility-safe bookmark payloads for MCP tools (never vault ciphertext).

use chrono::{DateTime, Utc};
use linkdean_core::{BookmarkArchiveView, BookmarkView, CategoryView, Tag};
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::HashMap;
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

/// Folder tree entry for `list_folders` (no vault rows).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    /// Nested path, e.g. `Work / Side projects`.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpFolderList {
    pub folders: Vec<McpFolder>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpTag {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpTagList {
    pub tags: Vec<McpTag>,
}

/// Archive metadata only — never file bodies or download URLs.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpArchiveStatus {
    pub format: String,
    pub status: String,
    pub bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct McpArchiveStatusList {
    pub archives: Vec<McpArchiveStatus>,
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

impl McpFolderList {
    pub fn from_categories(cats: &[CategoryView]) -> Self {
        let by_id: HashMap<Uuid, &CategoryView> = cats.iter().map(|c| (c.id, c)).collect();
        let folders = cats
            .iter()
            .map(|c| McpFolder {
                id: c.id.to_string(),
                name: c.name.clone(),
                parent_id: c.parent_id.map(|p| p.to_string()),
                path: folder_path(c, &by_id),
            })
            .collect();
        Self { folders }
    }
}

fn folder_path(cat: &CategoryView, by_id: &HashMap<Uuid, &CategoryView>) -> String {
    let mut parts = vec![cat.name.clone()];
    let mut parent = cat.parent_id;
    let mut guard = 0;
    while let Some(pid) = parent {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(p) = by_id.get(&pid) else {
            break;
        };
        parts.push(p.name.clone());
        parent = p.parent_id;
    }
    parts.reverse();
    parts.join(" / ")
}

impl McpTagList {
    pub fn from_tags(tags: &[Tag]) -> Self {
        Self {
            tags: tags
                .iter()
                .map(|t| McpTag {
                    name: t.name.clone(),
                })
                .collect(),
        }
    }
}

impl McpArchiveStatus {
    pub fn from_view(view: &BookmarkArchiveView) -> Self {
        Self {
            format: view.format.as_str().to_string(),
            status: view.status.as_str().to_string(),
            bytes: view.bytes,
            updated_at: view.captured_at,
        }
    }
}
