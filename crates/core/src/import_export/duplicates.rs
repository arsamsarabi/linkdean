//! What the import preview reports and what the person decides about each duplicate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// What to do with an imported bookmark whose URL is already in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAction {
    /// Drop the imported bookmark.
    Skip,
    /// Update the existing bookmark in place from the file.
    Replace,
    /// Keep the existing bookmark's values, fill the ones it lacks from the file, and add
    /// the file's tags and folders to its own.
    Merge,
    /// Create a second copy, as imports always did.
    #[default]
    Keep,
}

/// The person's choices, by index into the parsed file. Missing means `Keep` for every row.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateDecisions {
    #[serde(default)]
    pub default: DuplicateAction,
    #[serde(default)]
    pub overrides: HashMap<usize, DuplicateAction>,
}

impl DuplicateDecisions {
    pub fn action_for(&self, index: usize) -> DuplicateAction {
        self.overrides.get(&index).copied().unwrap_or(self.default)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateExisting {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    /// Full paths, `Work / Rust`.
    pub folders: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateIncoming {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportDuplicate {
    pub index: usize,
    pub url: String,
    pub existing: DuplicateExisting,
    pub incoming: DuplicateIncoming,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ImportFolderCounts {
    pub reused: usize,
    pub created: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportPreview {
    pub total: usize,
    pub new: usize,
    pub duplicates: Vec<ImportDuplicate>,
    pub folders: ImportFolderCounts,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_decisions_keep_every_row() {
        let d: DuplicateDecisions = serde_json::from_str("{}").unwrap();
        assert_eq!(d.action_for(0), DuplicateAction::Keep);
    }

    #[test]
    fn overrides_win_over_the_default_and_take_string_keys() {
        let d: DuplicateDecisions = serde_json::from_str(
            r#"{"default":"skip","overrides":{"3":"replace","4":"keep","5":"merge"}}"#,
        )
        .unwrap();
        assert_eq!(d.action_for(0), DuplicateAction::Skip);
        assert_eq!(d.action_for(3), DuplicateAction::Replace);
        assert_eq!(d.action_for(4), DuplicateAction::Keep);
        assert_eq!(d.action_for(5), DuplicateAction::Merge);
    }

    #[test]
    fn an_unknown_action_is_refused() {
        assert!(serde_json::from_str::<DuplicateDecisions>(r#"{"default":"combine"}"#).is_err());
    }
}
