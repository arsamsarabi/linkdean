//! Map an import's folders onto the person's existing folders by full path.

use linkdean_core::import_export::ImportFolderCounts;
use linkdean_core::{CreateCategory, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use super::categories::{create_category, list_owned_categories};

/// Full path of each parsed folder, root first. `parents[i]` is always lower than `i`.
pub(crate) fn parsed_paths(
    categories: &[CreateCategory],
    parents: &[Option<usize>],
) -> Vec<Vec<String>> {
    let mut paths: Vec<Vec<String>> = Vec::with_capacity(categories.len());
    for (cat, parent) in categories.iter().zip(parents) {
        let mut path = parent
            .and_then(|p| paths.get(p).cloned())
            .unwrap_or_default();
        path.push(cat.name.clone());
        paths.push(path);
    }
    paths
}

/// The person's live folders outside vaults, hidden ones included, by full path.
pub(crate) async fn existing_paths(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<HashMap<Vec<String>, Uuid>> {
    let cats = list_owned_categories(pool, user_id, true, false).await?;
    let by_id: HashMap<Uuid, (&str, Option<Uuid>)> = cats
        .iter()
        .map(|c| (c.id, (c.name.as_str(), c.parent_id)))
        .collect();
    let mut out = HashMap::with_capacity(cats.len());
    for c in &cats {
        let mut path = vec![c.name.clone()];
        let mut parent = c.parent_id;
        let mut guard = 0;
        // A parent outside the list (trashed, or in a vault) means this folder has no
        // reachable path, so it is never matched.
        let mut reachable = true;
        while let Some(pid) = parent {
            guard += 1;
            let Some((name, next)) = by_id.get(&pid) else {
                reachable = false;
                break;
            };
            if guard > 64 {
                reachable = false;
                break;
            }
            path.push((*name).to_string());
            parent = *next;
        }
        if reachable {
            path.reverse();
            // Two existing folders at one path: the first listed wins, consistently.
            out.entry(path).or_insert(c.id);
        }
    }
    Ok(out)
}

/// How many parsed folders would be reused and created, without writing anything.
pub(crate) async fn count_folders(
    pool: &PgPool,
    user_id: Uuid,
    categories: &[CreateCategory],
    parents: &[Option<usize>],
) -> Result<ImportFolderCounts> {
    let existing = existing_paths(pool, user_id).await?;
    let mut seen = std::collections::HashSet::new();
    let mut counts = ImportFolderCounts::default();
    for path in parsed_paths(categories, parents) {
        if !seen.insert(path.clone()) {
            continue;
        }
        if existing.contains_key(&path) {
            counts.reused += 1;
        } else {
            counts.created += 1;
        }
    }
    Ok(counts)
}

/// The folder id for each parsed folder, reusing existing ones and creating the rest.
pub(crate) async fn resolve_folders(
    pool: &PgPool,
    user_id: Uuid,
    categories: Vec<CreateCategory>,
    parents: Vec<Option<usize>>,
) -> Result<Vec<Uuid>> {
    let mut known = existing_paths(pool, user_id).await?;
    let paths = parsed_paths(&categories, &parents);
    let mut ids: Vec<Uuid> = Vec::with_capacity(categories.len());
    for ((cat, parent), path) in categories.into_iter().zip(parents).zip(paths) {
        if let Some(&id) = known.get(&path) {
            ids.push(id);
            continue;
        }
        let parent_id = parent.and_then(|i| ids.get(i).copied());
        let created = create_category(
            pool,
            user_id,
            CreateCategory {
                name: cat.name,
                parent_id,
                is_hidden: cat.is_hidden,
                vault_id: None,
                icon: cat.icon,
            },
        )
        .await?;
        known.insert(path, created.id);
        ids.push(created.id);
    }
    Ok(ids)
}
