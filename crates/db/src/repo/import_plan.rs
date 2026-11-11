//! Which imported bookmarks are already in the library, and what to do with each.

use linkdean_core::import_export::{
    DuplicateAction, DuplicateDecisions, DuplicateExisting, DuplicateIncoming, ImportDuplicate,
    ImportPreview, ParsedImport,
};
use linkdean_core::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::bookmarks::find_own_bookmarks_by_urls;
use super::import_folders::{count_folders, parsed_paths};

/// What happens to one parsed bookmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowPlan {
    Create,
    Skip,
    Replace(Uuid),
    Merge(Uuid),
}

/// One `RowPlan` per parsed bookmark, in file order.
#[derive(Debug, Clone, Default)]
pub struct ImportPlan {
    pub rows: Vec<RowPlan>,
}

impl ImportPlan {
    /// Bookmarks the import will create: what the bookmark quota is checked against.
    pub fn creates(&self) -> usize {
        self.rows.iter().filter(|r| **r == RowPlan::Create).count()
    }

    /// Every row created, as imports did before duplicates were checked.
    pub fn create_all(len: usize) -> Self {
        Self {
            rows: vec![RowPlan::Create; len],
        }
    }
}

fn urls(parsed: &ParsedImport) -> Vec<String> {
    parsed
        .bookmarks
        .iter()
        .map(|b| b.bookmark.url.clone())
        .collect()
}

/// Looks the duplicates up now, so the plan reflects the library at import time.
pub async fn plan_import(
    pool: &PgPool,
    user_id: Uuid,
    parsed: &ParsedImport,
    decisions: &DuplicateDecisions,
) -> Result<ImportPlan> {
    let existing = find_own_bookmarks_by_urls(pool, user_id, &urls(parsed)).await?;
    let rows = parsed
        .bookmarks
        .iter()
        .enumerate()
        .map(|(i, b)| match existing.get(&b.bookmark.url) {
            None => RowPlan::Create,
            Some(view) => match decisions.action_for(i) {
                DuplicateAction::Keep => RowPlan::Create,
                DuplicateAction::Skip => RowPlan::Skip,
                DuplicateAction::Replace => RowPlan::Replace(view.id),
                DuplicateAction::Merge => RowPlan::Merge(view.id),
            },
        })
        .collect();
    Ok(ImportPlan { rows })
}

/// What the import would do, without writing anything.
pub async fn preview_import(
    pool: &PgPool,
    user_id: Uuid,
    parsed: &ParsedImport,
) -> Result<ImportPreview> {
    let existing = find_own_bookmarks_by_urls(pool, user_id, &urls(parsed)).await?;
    let paths: Vec<String> = parsed_paths(&parsed.categories, &parsed.category_parents)
        .into_iter()
        .map(|p| p.join(" / "))
        .collect();
    let duplicates: Vec<ImportDuplicate> = parsed
        .bookmarks
        .iter()
        .enumerate()
        .filter_map(|(index, b)| {
            let view = existing.get(&b.bookmark.url)?;
            Some(ImportDuplicate {
                index,
                url: b.bookmark.url.clone(),
                existing: DuplicateExisting {
                    id: view.id,
                    title: view.title.clone(),
                    description: view.description.clone(),
                    tags: view.tags.clone(),
                    folders: view.categories.iter().map(|c| c.path.clone()).collect(),
                    updated_at: view.updated_at,
                },
                incoming: DuplicateIncoming {
                    title: b.bookmark.title.clone(),
                    description: b.bookmark.description.clone(),
                    tags: b.bookmark.tags.clone(),
                    folders: b
                        .categories
                        .iter()
                        .filter_map(|&i| paths.get(i).cloned())
                        .collect(),
                },
            })
        })
        .collect();
    let folders =
        count_folders(pool, user_id, &parsed.categories, &parsed.category_parents).await?;
    Ok(ImportPreview {
        total: parsed.bookmarks.len(),
        new: parsed.bookmarks.len() - duplicates.len(),
        duplicates,
        folders,
    })
}
