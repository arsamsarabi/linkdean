# Duplicates on Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before an import writes anything, show the person which bookmarks are already in their library and let them Skip, Replace or Import anyway; reuse existing folders by path.

**Architecture:** Spec: [import-duplicates.md](./import-duplicates.md). The db crate gains folder reuse by full path, a duplicate lookup by exact URL, an `ImportPlan` built from the person's decisions, and plan-aware apply. The API gains `POST /api/import/preview` and an optional `duplicates` field on `POST /api/import`. The web app previews first and opens a review drawer when there are duplicates. No server state between the two calls; no migration.

**Tech Stack:** Rust (axum, sqlx, Postgres), React 19 + Mantine 9 + TanStack Query, Vitest + Testing Library.

**Repo rules that apply to every task** (from the `linkdean-repo-workflow` skill):
- Work on branch `feat/import-duplicates`. Never commit to `main`. Commit and push after each green task.
- Stage only files you changed. Never `git add -A`. Never stage `docs/product/ideas.md`, `graphify-out/` or `brand/graphics/`.
- Rust tests: `cargo test -j 2`. DB tests need `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test` (throwaway `pg-check` container must be up). Check the `test result` counts and that no `skipping ... DATABASE_URL unset` line printed.
- Rust fmt: `cargo fmt --all -- --check` in a throwaway `rust:<ver>` container, `<ver>` from `toolchain:` in `.github/workflows/ci.yml`: `docker run --rm -v "$PWD":/w -w /w rust:<ver> sh -c 'rustup component add rustfmt >/dev/null && cargo fmt --all -- --check'`. Remove nothing else; the container is `--rm`.
- Web: from `apps/web`, `pnpm exec prettier --check <changed files>`, `pnpm exec tsc -b`, `pnpm lint`, `pnpm exec vitest run <paths>`.
- After code changes, `graphify update .` from the repo root before reporting.

---

## File structure

| File | Responsibility |
| --- | --- |
| `crates/core/src/import_export/duplicates.rs` (new) | Wire types: `DuplicateAction`, `DuplicateDecisions`, `ImportPreview` and its parts. Pure, serde only. |
| `crates/core/src/import_export/mod.rs` | `pub mod duplicates;` and re-exports. |
| `crates/db/src/repo/import_folders.rs` (new) | Map the file's folders onto existing folders by full path; create only missing ones. |
| `crates/db/src/repo/import_plan.rs` (new) | Find exact-URL duplicates, build `ImportPlan` from decisions, build `ImportPreview`. |
| `crates/db/src/repo/import.rs` | Apply uses folder reuse and the plan; `ImportCounts` gains `skipped`, `replaced`. |
| `crates/db/src/repo/bookmarks.rs` | `find_own_bookmarks_by_urls`: latest live non-vault bookmark per URL. |
| `crates/db/src/repo/mod.rs` | Register and re-export the new modules. |
| `crates/db/tests/import_duplicates.rs` (new) | DB tests for preview, plan, apply and folder reuse. |
| `crates/api/src/rate_limit.rs` | `import_preview_limit`. |
| `crates/api/src/routes/import_export/mod.rs` | Preview route; `duplicates` on import; quota counts only creates; new response fields. |
| `apps/web/src/schemas/bookmarks/bookmarks.ts` | `importPreviewSchema`; `importResultSchema` gains `skipped`, `replaced`. |
| `apps/web/src/api/api.ts` | `importPreview`, `import` with optional decisions. |
| `apps/web/src/hooks/import-export/useImportExport.ts` | `useImportPreview`; `useImportBookmarks` passes decisions. |
| `apps/web/src/components/import-export/duplicateDecisions.ts` (new) | Pure: effective action per row, and the minimal `{default, overrides}` body. |
| `apps/web/src/components/import-export/ImportDuplicatesDrawer.tsx` (new) | The review drawer. |
| `apps/web/src/components/import-export/ImportExportPanel.tsx` | Preview first, open the drawer, import with decisions. |
| Docs | `docs/product/features.md`, `docs/product/roadmap.md`, `docs/CHANGELOG.md`, rebuilt `deploy/public-beta/static/docs`. |

---

### Task 1: Core wire types

**Files:**
- Create: `crates/core/src/import_export/duplicates.rs`
- Modify: `crates/core/src/import_export/mod.rs` (top of file, after the `use` block)

- [ ] **Step 1: Write the types and their tests**

`crates/core/src/import_export/duplicates.rs`:

```rust
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
        let d: DuplicateDecisions =
            serde_json::from_str(r#"{"default":"skip","overrides":{"3":"replace","4":"keep"}}"#)
                .unwrap();
        assert_eq!(d.action_for(0), DuplicateAction::Skip);
        assert_eq!(d.action_for(3), DuplicateAction::Replace);
        assert_eq!(d.action_for(4), DuplicateAction::Keep);
    }

    #[test]
    fn an_unknown_action_is_refused() {
        assert!(serde_json::from_str::<DuplicateDecisions>(r#"{"default":"merge"}"#).is_err());
    }
}
```

In `crates/core/src/import_export/mod.rs`, after the `use crate::{…};` block:

```rust
pub mod duplicates;
pub use duplicates::*;
```

`serde_json` is already a dependency of `linkdean-core`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -j 2 -p linkdean-core duplicates`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/core/src/import_export/duplicates.rs crates/core/src/import_export/mod.rs
git commit -m "Add the wire types for import duplicate previews and decisions"
git push
```

---

### Task 2: Reuse existing folders by full path

**Files:**
- Create: `crates/db/src/repo/import_folders.rs`
- Modify: `crates/db/src/repo/import.rs:83-100` (the folder loop in `apply_parsed_import_counted`)
- Modify: `crates/db/src/repo/mod.rs` (register module)
- Create: `crates/db/tests/import_duplicates.rs`

- [ ] **Step 1: Write the failing tests**

`crates/db/tests/import_duplicates.rs`:

```rust
//! Import reuses folders by path, and lets the person decide about bookmarks already in
//! the library.
//!
//! Requires Postgres. Set `DATABASE_URL`. Skips when unset.

use linkdean_core::import_export::parse_netscape;
use linkdean_core::{CreateBookmark, CreateCategory, User, VaultKeyring};
use linkdean_db::{self as db};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool_or_skip() -> Option<PgPool> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping import duplicate tests: DATABASE_URL unset");
        return None;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(pool)
}

async fn person(pool: &PgPool) -> User {
    let email = format!("dup-{}@import.test", Uuid::new_v4());
    db::create_user(pool, &email, Some("password-long"), None, None, None)
        .await
        .expect("user")
}

async fn folder(pool: &PgPool, owner: &User, name: &str, parent_id: Option<Uuid>) -> Uuid {
    db::create_category(
        pool,
        owner.id,
        CreateCategory {
            name: name.into(),
            parent_id,
            is_hidden: false,
            vault_id: None,
            icon: None,
        },
    )
    .await
    .expect("folder")
    .id
}

async fn folder_count(pool: &PgPool, owner: &User) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE owner_user_id = $1 AND deleted_at IS NULL")
        .bind(owner.id)
        .fetch_one(pool)
        .await
        .expect("count")
}

fn html(body: &str) -> String {
    format!("<DL><p>\n{body}</DL><p>\n")
}

fn a(url: &str, title: &str) -> String {
    format!("<DT><A HREF=\"{url}\">{title}</A>\n")
}

fn h3(name: &str, inner: &str) -> String {
    format!("<DT><H3>{name}</H3>\n<DL><p>\n{inner}</DL><p>\n")
}

#[tokio::test]
async fn importing_the_same_file_twice_reuses_its_folders() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let file = html(&h3("Work", &h3("Rust", &a("https://f1.example", "F1"))));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap()).await.unwrap();
    assert_eq!(folder_count(&pool, &me).await, 2);
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap()).await.unwrap();
    assert_eq!(folder_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn a_new_child_goes_under_a_reused_parent() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let work = folder(&pool, &me, "Work", None).await;
    let file = html(&h3("Work", &h3("Go", &a("https://f2.example", "F2"))));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap()).await.unwrap();
    let (parent,): (Option<Uuid>,) =
        sqlx::query_as("SELECT parent_id FROM categories WHERE owner_user_id = $1 AND name = 'Go'")
            .bind(me.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(parent, Some(work));
    assert_eq!(folder_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn same_name_siblings_in_a_file_share_one_folder() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let file = html(&format!(
        "{}{}",
        h3("Reading", &a("https://f3.example", "F3")),
        h3("Reading", &a("https://f4.example", "F4"))
    ));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap()).await.unwrap();
    assert_eq!(folder_count(&pool, &me).await, 1);
}

#[tokio::test]
async fn names_match_exactly_and_vault_folders_are_not_reused() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    folder(&pool, &me, "work", None).await;
    let file = html(&h3("Work", &a("https://f5.example", "F5")));
    db::apply_parsed_import(&pool, me.id, parse_netscape(&file).unwrap()).await.unwrap();
    assert_eq!(folder_count(&pool, &me).await, 2);
}
```

Vault folders: `list_owned_categories(.., include_vault = false)` already leaves them out, so the reuse code below never sees them. The test above covers case; vault exclusion is covered by construction.

- [ ] **Step 2: Run to see them fail**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -j 2 -p linkdean-db --test import_duplicates`
Expected: `importing_the_same_file_twice_reuses_its_folders`, `a_new_child_goes_under_a_reused_parent` and `same_name_siblings_in_a_file_share_one_folder` FAIL on folder counts (4, 3, 2). The `names_match_exactly…` test passes already.

- [ ] **Step 3: Implement folder reuse**

`crates/db/src/repo/import_folders.rs`:

```rust
//! Map an import's folders onto the person's existing folders by full path.

use linkdean_core::{CreateCategory, ImportFolderCounts, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use super::categories::{create_category, list_owned_categories};

/// Full path of each parsed folder, root first. `parents[i]` is always lower than `i`.
pub(crate) fn parsed_paths(categories: &[CreateCategory], parents: &[Option<usize>]) -> Vec<Vec<String>> {
    let mut paths: Vec<Vec<String>> = Vec::with_capacity(categories.len());
    for (cat, parent) in categories.iter().zip(parents) {
        let mut path = parent.and_then(|p| paths.get(p).cloned()).unwrap_or_default();
        path.push(cat.name.clone());
        paths.push(path);
    }
    paths
}

/// The person's live folders outside vaults, hidden ones included, by full path.
pub(crate) async fn existing_paths(pool: &PgPool, user_id: Uuid) -> Result<HashMap<Vec<String>, Uuid>> {
    let cats = list_owned_categories(pool, user_id, true, false).await?;
    let by_id: HashMap<Uuid, (&str, Option<Uuid>)> =
        cats.iter().map(|c| (c.id, (c.name.as_str(), c.parent_id))).collect();
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
```

In `crates/db/src/repo/mod.rs`, next to `mod import;`:

```rust
mod import_folders;
```

(It stays crate-private; nothing re-exports it.)

In `crates/db/src/repo/import.rs`, replace the folder loop at the top of `apply_parsed_import_counted` (lines 83–100, from `// Parsers list each parent…` to the closing `}` of the `for`) with:

```rust
    let folder_ids =
        super::import_folders::resolve_folders(pool, user_id, parsed.categories, parsed.category_parents)
            .await?;
```

Remove `create_category` from the `use super::categories::{…}` line if it is now unused; keep `list_owned_categories` (export uses it).

- [ ] **Step 4: Run the new and existing import tests**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -j 2 -p linkdean-db --test import_duplicates --test export_import`
Expected: all pass, no `skipping` line. `html_import_keeps_slashes_and_same_name_folders_apart` still passes: `AC/DC / Live` and `Rock / Live` are different paths.

- [ ] **Step 5: fmt, commit, push**

```bash
docker run --rm -v "$PWD":/w -w /w rust:<ver> sh -c 'rustup component add rustfmt >/dev/null && cargo fmt --all -- --check'
git add crates/db/src/repo/import_folders.rs crates/db/src/repo/import.rs crates/db/src/repo/mod.rs crates/db/tests/import_duplicates.rs
git commit -m "Reuse existing folders by path when importing"
git push
```

---

### Task 3: Find duplicates and build the preview

**Files:**
- Modify: `crates/db/src/repo/bookmarks.rs` (after `find_own_bookmark_by_url`, ~line 1955)
- Create: `crates/db/src/repo/import_plan.rs`
- Modify: `crates/db/src/repo/mod.rs` (register + `pub use import_plan::*;`)
- Test: `crates/db/tests/import_duplicates.rs`

- [ ] **Step 1: Write the failing tests** (append to `crates/db/tests/import_duplicates.rs`)

```rust
async fn bookmark(pool: &PgPool, owner: &User, url: &str, title: &str, folders: Vec<Uuid>) -> Uuid {
    db::create_bookmark(
        pool,
        owner.id,
        CreateBookmark {
            url: url.into(),
            title: Some(title.into()),
            category_ids: folders,
            tags: vec!["old".into()],
            ..Default::default()
        },
        &VaultKeyring::default(),
    )
    .await
    .expect("bookmark")
    .id
}

async fn view_of(pool: &PgPool, owner: &User, id: Uuid) -> linkdean_core::BookmarkView {
    let raw = db::get_bookmark_raw(pool, owner.id, id).await.expect("raw");
    db::bookmark_to_view(pool, raw, &VaultKeyring::default(), owner.id)
        .await
        .expect("view")
        .expect("visible")
}

async fn bookmark_count(pool: &PgPool, owner: &User) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM bookmarks WHERE owner_user_id = $1 AND deleted_at IS NULL")
        .bind(owner.id)
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn preview_lists_exact_url_duplicates_and_writes_nothing() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let work = folder(&pool, &me, "Work", None).await;
    let url = format!("https://p.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Mine", vec![work]).await;
    let file = html(&format!(
        "{}{}{}",
        a("https://fresh.example", "Fresh"),
        h3("Work", &a(&url, "Theirs")),
        a(&format!("{url}/"), "Slash differs")
    ));
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&file).unwrap()).await.unwrap();

    assert_eq!((preview.total, preview.new), (3, 2));
    assert_eq!(preview.duplicates.len(), 1);
    let dup = &preview.duplicates[0];
    assert_eq!((dup.index, dup.url.as_str()), (1, url.as_str()));
    assert_eq!(dup.existing.id, id);
    assert_eq!(dup.existing.title, "Mine");
    assert_eq!(dup.existing.folders, vec!["Work".to_string()]);
    assert_eq!(dup.incoming.title.as_deref(), Some("Theirs"));
    assert_eq!(dup.incoming.folders, vec!["Work".to_string()]);
    assert_eq!((preview.folders.reused, preview.folders.created), (1, 0));
    assert_eq!(bookmark_count(&pool, &me).await, 1);
}

#[tokio::test]
async fn trashed_and_vault_bookmarks_are_not_duplicates() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let url = format!("https://t.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Gone", vec![]).await;
    sqlx::query("UPDATE bookmarks SET deleted_at = now() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let vault_url = format!("https://v.example/{}", Uuid::new_v4());
    let vid = bookmark(&pool, &me, &vault_url, "Vaulted", vec![]).await;
    sqlx::query("UPDATE bookmarks SET is_vault = TRUE WHERE id = $1")
        .bind(vid)
        .execute(&pool)
        .await
        .unwrap();
    let file = html(&format!("{}{}", a(&url, "T"), a(&vault_url, "V")));
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&file).unwrap()).await.unwrap();
    assert!(preview.duplicates.is_empty());
    assert_eq!(preview.new, 2);
}

#[tokio::test]
async fn several_copies_in_the_library_report_the_latest() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let url = format!("https://l.example/{}", Uuid::new_v4());
    let older = bookmark(&pool, &me, &url, "Older", vec![]).await;
    let newer = bookmark(&pool, &me, &url, "Newer", vec![]).await;
    sqlx::query("UPDATE bookmarks SET updated_at = now() - interval '1 day' WHERE id = $1")
        .bind(older)
        .execute(&pool)
        .await
        .unwrap();
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&html(&a(&url, "X"))).unwrap())
        .await
        .unwrap();
    assert_eq!(preview.duplicates[0].existing.id, newer);
}
```

The vault test flips `is_vault` with SQL only to exercise the filter; it never reads the row back through the vault code.

- [ ] **Step 2: Run to see them fail**

Run: `DATABASE_URL=… cargo test -j 2 -p linkdean-db --test import_duplicates`
Expected: compile error, `preview_import` not found.

- [ ] **Step 3: Implement the lookup**

In `crates/db/src/repo/bookmarks.rs`, after `find_own_bookmark_by_url`:

```rust
/// For each of `urls`, the person's own live bookmark at exactly that URL outside any
/// vault, the most recently updated when there are several. Import uses it to find
/// duplicates, with the same rule as quick save.
pub async fn find_own_bookmarks_by_urls(
    pool: &PgPool,
    user_id: Uuid,
    urls: &[String],
) -> Result<HashMap<String, BookmarkView>> {
    if urls.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, Bookmark>(
        r#"
        SELECT DISTINCT ON (url) * FROM bookmarks
        WHERE owner_user_id = $1 AND is_vault = FALSE AND deleted_at IS NULL AND url = ANY($2)
        ORDER BY url, updated_at DESC, created_at DESC
        "#,
    )
    .bind(user_id)
    .bind(urls)
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(e.to_string()))?;
    let none = VaultKeyring::default();
    let views = bookmarks_to_views(pool, rows, user_id, |_| &none).await?;
    Ok(views.into_iter().map(|v| (v.url.clone(), v)).collect())
}
```

`bookmarks.rs` already imports `std::collections::HashMap`.

- [ ] **Step 4: Implement the plan and the preview**

`crates/db/src/repo/import_plan.rs`:

```rust
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
        Self { rows: vec![RowPlan::Create; len] }
    }
}

fn urls(parsed: &ParsedImport) -> Vec<String> {
    parsed.bookmarks.iter().map(|b| b.bookmark.url.clone()).collect()
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
            },
        })
        .collect();
    Ok(ImportPlan { rows })
}

/// What the import would do, without writing anything.
pub async fn preview_import(pool: &PgPool, user_id: Uuid, parsed: &ParsedImport) -> Result<ImportPreview> {
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
                    folders: b.categories.iter().filter_map(|&i| paths.get(i).cloned()).collect(),
                },
            })
        })
        .collect();
    let folders = count_folders(pool, user_id, &parsed.categories, &parsed.category_parents).await?;
    Ok(ImportPreview {
        total: parsed.bookmarks.len(),
        new: parsed.bookmarks.len() - duplicates.len(),
        duplicates,
        folders,
    })
}
```

In `crates/db/src/repo/mod.rs`: add `mod import_plan;` beside `mod import;`, and `pub use import_plan::*;` beside `pub use import::*;`. `repo/mod.rs` has `pub use bookmarks::*;`, so `find_own_bookmarks_by_urls` is exported as `db::find_own_bookmarks_by_urls`.

- [ ] **Step 5: Run the tests**

Run: `DATABASE_URL=… cargo test -j 2 -p linkdean-db --test import_duplicates`
Expected: all pass, no `skipping` line.

- [ ] **Step 6: fmt, commit, push**

```bash
git add crates/db/src/repo/bookmarks.rs crates/db/src/repo/import_plan.rs crates/db/src/repo/mod.rs crates/db/tests/import_duplicates.rs
git commit -m "Preview which imported bookmarks are already in the library"
git push
```

---

### Task 4: Apply the plan (skip, replace, keep)

**Files:**
- Modify: `crates/db/src/repo/import.rs` (`ImportCounts`, `apply_parsed_import`, `apply_parsed_import_counted`)
- Modify: `crates/db/tests/export_import.rs:202` (call site gains a plan)
- Test: `crates/db/tests/import_duplicates.rs`

- [ ] **Step 1: Write the failing tests** (append; change the first `use` line to `use linkdean_core::import_export::{parse_netscape, DuplicateAction, DuplicateDecisions};`)

```rust
async fn import_with(pool: &PgPool, me: &User, file: &str, decisions: DuplicateDecisions) -> db::ImportCounts {
    let parsed = parse_netscape(file).unwrap();
    let plan = db::plan_import(pool, me.id, &parsed, &decisions).await.unwrap();
    let mut counts = db::ImportCounts::default();
    db::apply_parsed_import_counted(pool, me.id, parsed, &plan, &mut counts).await.unwrap();
    counts
}

fn all(action: DuplicateAction) -> DuplicateDecisions {
    DuplicateDecisions { default: action, ..Default::default() }
}

#[tokio::test]
async fn skip_creates_nothing_for_the_duplicate() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let url = format!("https://s.example/{}", Uuid::new_v4());
    bookmark(&pool, &me, &url, "Mine", vec![]).await;
    let file = html(&format!("{}{}", a(&url, "Theirs"), a("https://s-new.example", "New")));
    let c = import_with(&pool, &me, &file, all(DuplicateAction::Skip)).await;
    assert_eq!((c.total, c.imported, c.skipped, c.replaced), (2, 1, 1, 0));
    assert_eq!(bookmark_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn keep_creates_a_second_copy_and_is_the_default() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let url = format!("https://k.example/{}", Uuid::new_v4());
    bookmark(&pool, &me, &url, "Mine", vec![]).await;
    let c = import_with(&pool, &me, &html(&a(&url, "Theirs")), DuplicateDecisions::default()).await;
    assert_eq!((c.imported, c.skipped, c.replaced), (1, 0, 0));
    assert_eq!(bookmark_count(&pool, &me).await, 2);
}

#[tokio::test]
async fn replace_updates_in_place_and_keeps_the_id() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let old = folder(&pool, &me, "Old", None).await;
    let url = format!("https://r.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Mine", vec![old]).await;
    sqlx::query("UPDATE bookmarks SET is_favorite = TRUE WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let file = html(&h3("New", &format!(
        "<DT><A HREF=\"{url}\" TAGS=\"fresh\">Theirs</A>\n<DD>Their notes\n"
    )));
    let c = import_with(&pool, &me, &file, all(DuplicateAction::Replace)).await;
    assert_eq!((c.imported, c.replaced), (0, 1));
    assert_eq!(bookmark_count(&pool, &me).await, 1);

    let view = view_of(&pool, &me, id).await;
    assert_eq!(view.title, "Theirs");
    assert_eq!(view.description.as_deref(), Some("Their notes"));
    assert_eq!(view.tags, vec!["fresh".to_string()]);
    assert_eq!(view.categories.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), vec!["New"]);
    assert!(view.is_favorite);
}

#[tokio::test]
async fn replace_keeps_values_the_file_does_not_have() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let old = folder(&pool, &me, "Old", None).await;
    let url = format!("https://r2.example/{}", Uuid::new_v4());
    let id = bookmark(&pool, &me, &url, "Mine", vec![old]).await;
    // No title, no tags, no folder in the file.
    let c = import_with(&pool, &me, &html(&format!("<DT><A HREF=\"{url}\"></A>\n")), all(DuplicateAction::Replace)).await;
    assert_eq!(c.replaced, 1);
    let view = view_of(&pool, &me, id).await;
    assert_eq!(view.title, "Mine");
    assert_eq!(view.tags, vec!["old".to_string()]);
    assert_eq!(view.categories.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(), vec!["Old"]);
}

#[tokio::test]
async fn an_override_beats_the_default_and_the_plan_counts_creates() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let u1 = format!("https://o1.example/{}", Uuid::new_v4());
    let u2 = format!("https://o2.example/{}", Uuid::new_v4());
    bookmark(&pool, &me, &u1, "One", vec![]).await;
    bookmark(&pool, &me, &u2, "Two", vec![]).await;
    let file = html(&format!("{}{}{}", a(&u1, "1"), a(&u2, "2"), a("https://o3.example", "3")));
    let parsed = parse_netscape(&file).unwrap();
    let mut decisions = all(DuplicateAction::Skip);
    decisions.overrides.insert(1, DuplicateAction::Keep);
    let plan = db::plan_import(&pool, me.id, &parsed, &decisions).await.unwrap();
    assert_eq!(plan.creates(), 2);
}

#[tokio::test]
async fn a_bookmark_added_after_the_preview_gets_the_default() {
    let Some(pool) = pool_or_skip().await else { return };
    let me = person(&pool).await;
    let url = format!("https://late.example/{}", Uuid::new_v4());
    let file = html(&a(&url, "Late"));
    let preview = db::preview_import(&pool, me.id, &parse_netscape(&file).unwrap()).await.unwrap();
    assert!(preview.duplicates.is_empty());
    bookmark(&pool, &me, &url, "Added meanwhile", vec![]).await;
    let c = import_with(&pool, &me, &file, all(DuplicateAction::Skip)).await;
    assert_eq!((c.imported, c.skipped), (0, 1));
}
```

The Netscape parser reads `TAGS="…"` as tags and `<DD>` as the description (`crates/core/src/import_export/mod.rs:179, 236`), so the file in `replace_updates_in_place…` exercises both.

- [ ] **Step 2: Run to see them fail**

Run: `DATABASE_URL=… cargo test -j 2 -p linkdean-db --test import_duplicates`
Expected: compile errors: `skipped`/`replaced` fields, `apply_parsed_import_counted` arity.

- [ ] **Step 3: Implement**

In `crates/db/src/repo/import.rs`:

```rust
/// How far an import got. Filled in as it goes, so a failed import still reports it.
#[derive(Debug, Default, Clone, Copy)]
pub struct ImportCounts {
    pub total: i32,
    /// Bookmarks created.
    pub imported: i32,
    pub failed: i32,
    /// Duplicates dropped by Skip.
    pub skipped: i32,
    /// Existing bookmarks updated by Replace.
    pub replaced: i32,
}

/// An import with no duplicate decisions: every bookmark is created, as before.
pub async fn apply_parsed_import(
    pool: &PgPool,
    user_id: Uuid,
    parsed: ParsedImport,
) -> Result<(i32, i32, i32)> {
    let mut counts = ImportCounts::default();
    let plan = super::import_plan::ImportPlan::create_all(parsed.bookmarks.len());
    apply_parsed_import_counted(pool, user_id, parsed, &plan, &mut counts).await?;
    Ok((counts.total, counts.imported, counts.failed))
}
```

Change `apply_parsed_import_counted` to take `plan: &super::import_plan::ImportPlan` after `parsed`, and replace the bookmark loop (`for item in parsed.bookmarks { … }`) with:

```rust
    counts.total = parsed.bookmarks.len() as i32;
    let mut new_ids: HashMap<Uuid, Uuid> = HashMap::new();
    for (index, item) in parsed.bookmarks.into_iter().enumerate() {
        let category_ids: Vec<Uuid> = item
            .categories
            .iter()
            .filter_map(|&i| folder_ids.get(i).copied())
            .collect();
        match plan.rows.get(index).copied().unwrap_or(RowPlan::Create) {
            RowPlan::Skip => {
                counts.skipped += 1;
            }
            RowPlan::Replace(existing_id) => {
                match replace_from_import(pool, user_id, existing_id, &item.bookmark, category_ids).await {
                    Ok(()) => {
                        counts.replaced += 1;
                        if let Some(source) = item.source_id {
                            new_ids.insert(source, existing_id);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(?e, "import replace failed");
                        counts.failed += 1;
                    }
                }
            }
            RowPlan::Create => {
                let mut bm = item.bookmark;
                bm.category_id = None;
                bm.category_ids = category_ids;
                let created = match create_bookmark(pool, user_id, bm, &VaultKeyring::default()).await {
                    Ok(view) => view,
                    Err(e) => {
                        tracing::warn!(?e, "import bookmark failed");
                        counts.failed += 1;
                        continue;
                    }
                };
                counts.imported += 1;
                if let Some(source) = item.source_id {
                    new_ids.insert(source, created.id);
                }
                if item.is_private || item.created_at.is_some() || item.updated_at.is_some() {
                    // (unchanged UPDATE bookmarks SET is_private … block, using created.id)
                }
            }
        }
    }
```

Move the existing `if item.is_private || …` SQL block into the `Create` arm unchanged. Add the helper below `apply_parsed_import_counted`:

```rust
/// Replace: the file's title, description, tags and folders win when the file has them.
/// Everything else on the existing bookmark stays.
async fn replace_from_import(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    incoming: &linkdean_core::CreateBookmark,
    category_ids: Vec<Uuid>,
) -> Result<()> {
    let non_empty = |s: &Option<String>| s.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
    let req = linkdean_core::UpdateBookmark {
        title: non_empty(&incoming.title),
        description: non_empty(&incoming.description).map(Some),
        tags: (!incoming.tags.is_empty()).then(|| incoming.tags.clone()),
        category_ids: (!category_ids.is_empty()).then_some(category_ids),
        ..Default::default()
    };
    if req.title.is_none() && req.description.is_none() && req.tags.is_none() && req.category_ids.is_none() {
        // Nothing in the file to take; the bookmark already matches it.
        return Ok(());
    }
    super::bookmarks::update_bookmark(pool, user_id, id, req, &VaultKeyring::default()).await?;
    Ok(())
}
```

Add `use super::import_plan::RowPlan;` at the top of `import.rs`.

Update the one other caller, `crates/db/tests/export_import.rs` (`a_failed_import_reports_what_it_got_through`):

```rust
    let plan = db::ImportPlan::create_all(parsed.bookmarks.len());
    let result = db::apply_parsed_import_counted(&pool, to.id, parsed, &plan, &mut counts).await;
```

- [ ] **Step 4: Run all db import tests**

Run: `DATABASE_URL=… cargo test -j 2 -p linkdean-db --test import_duplicates --test export_import --test duplicates`
Expected: all pass, no `skipping` line.

- [ ] **Step 5: fmt, commit, push**

```bash
git add crates/db/src/repo/import.rs crates/db/tests/import_duplicates.rs crates/db/tests/export_import.rs
git commit -m "Apply skip, replace and keep decisions when importing"
git push
```

---

### Task 5: API routes

**Files:**
- Modify: `crates/api/src/rate_limit.rs:24-30` (constant) and `:116-118` (limit fn)
- Modify: `crates/api/src/routes/import_export/mod.rs`

- [ ] **Step 1: Rate limit**

In `rate_limit.rs`, beside `IMPORT_USER`:

```rust
/// Import previews one user may ask for per window. Each parses the whole file, but writes
/// nothing, so it has room for a few tries per import.
const IMPORT_PREVIEW_USER: u32 = 30;
```

Beside `import_limit`:

```rust
pub fn import_preview_limit(user_id: uuid::Uuid) -> [Limit; 1] {
    [(format!("import-preview:{user_id}"), IMPORT_PREVIEW_USER)]
}
```

- [ ] **Step 2: Shared parse helper, preview route, decisions on import**

In `routes/import_export/mod.rs`:

Router:

```rust
        .route(
            "/import/preview",
            post(preview).layer(DefaultBodyLimit::max(MAX_IMPORT_BODY)),
        )
```

Body:

```rust
#[derive(Deserialize)]
pub struct ImportBody {
    pub format: String,
    pub content: String,
    /// What to do with bookmarks already in the library. Missing: create them all, as before.
    #[serde(default)]
    pub duplicates: linkdean_core::import_export::DuplicateDecisions,
}
```

Move the size check, format match and `spawn_blocking` parse out of `import` into:

```rust
/// Checks the size, picks the parser and parses off the async runtime.
async fn parse_import(
    state: &AppState,
    format: &str,
    content: String,
) -> Result<(String, linkdean_core::import_export::ParsedImport, linkdean_core::ResolvedLimits), Error> {
    let limits = crate::quotas::limits_for(&state.pool).await?;
    let max_bytes = crate::quotas::effective_import_max(&limits);
    if content.len() > max_bytes {
        return Err(Error::QuotaExceeded {
            meter: db::METER_IMPORT,
            limit: max_bytes as i64,
            used: content.len() as i64,
        });
    }
    let format = format.to_ascii_lowercase();
    let parser = match format.as_str() {
        "netscape" | "html" => parse_netscape,
        "json" | "linkdean" => parse_linkdean_json,
        "pocket" => parse_pocket_csv,
        "raindrop" => parse_raindrop_csv,
        _ => return Err(Error::BadRequest("format must be netscape|json|pocket|raindrop".into())),
    };
    let parsed = tokio::task::spawn_blocking(move || parser(&content))
        .await
        .map_err(|e| Error::msg(format!("import parse: {e}")))??;
    Ok((format, parsed, limits))
}
```

`crate::quotas::limits_for` returns `linkdean_core::ResolvedLimits`. Match the existing error conversions in the file (`.into()` to the API error type); keep the same `Error` type the handler used before.

Preview handler:

```rust
async fn preview(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ImportBody>,
) -> ApiResult<Json<linkdean_core::import_export::ImportPreview>> {
    let rate = crate::rate_limit::import_preview_limit(user.id);
    state.limiter.check(&rate)?;
    let _slot = ImportSlot::acquire(user.id)?;
    state.limiter.record(&rate);
    let (_, parsed, _) = parse_import(&state, &body.format, body.content).await?;
    Ok(Json(db::preview_import(&state.pool, user.id, &parsed).await?))
}
```

In `import`, after `_slot` and `record`:

```rust
    let (format, parsed, limits) = parse_import(&state, &body.format, body.content).await?;
    let plan = db::plan_import(&state.pool, user.id, &parsed, &body.duplicates).await?;

    // Refuse the whole import when the bookmark gauge would overflow. Skipped and replaced
    // duplicates add nothing.
    db::refuse_bookmark_quota(&state.pool, user.id, &limits, plan.creates() as i64).await?;

    let job = db::create_import_job(&state.pool, user.id, &format).await?;
    let mut counts = db::ImportCounts::default();
    let applied = db::apply_parsed_import_counted(&state.pool, user.id, parsed, &plan, &mut counts).await;
```

Response:

```rust
    Ok(Json(json!({
        "job_id": job.id,
        "total": counts.total,
        "imported": counts.imported,
        "failed": counts.failed,
        "skipped": counts.skipped,
        "replaced": counts.replaced,
        "status": job.status,
    })))
```

(Drop the `let (total, imported, failed) = …` line it replaces.)

- [ ] **Step 3: Build and run the API crate tests**

Run: `cargo build -j 2 -p linkdean-api && cargo test -j 2 -p linkdean-api`
Expected: builds, existing tests pass.

- [ ] **Step 4: Smoke test against the dev API** (dev stack running; cargo-watch rebuilds it)

Use the `linkdean-throwaway-smoke-stack` skill if the dev API is not available. With a session cookie or API key for a test account in `$AUTH` (header line, e.g. `Authorization: Bearer …`):

```bash
FILE='<DL><p><DT><A HREF="https://smoke.example/a">A</A></DL><p>'
curl -s -H "$AUTH" -H 'content-type: application/json' localhost:8080/api/import -d "$(jq -n --arg c "$FILE" '{format:"netscape",content:$c}')" | jq .
curl -s -H "$AUTH" -H 'content-type: application/json' localhost:8080/api/import/preview -d "$(jq -n --arg c "$FILE" '{format:"netscape",content:$c}')" | jq '.total,.new,(.duplicates|length)'
curl -s -H "$AUTH" -H 'content-type: application/json' localhost:8080/api/import -d "$(jq -n --arg c "$FILE" '{format:"netscape",content:$c,duplicates:{default:"skip"}}')" | jq '.imported,.skipped'
```

Expected: first import `imported: 1`; preview `1, 0, 1`; second import `0, 1`. Delete the test bookmark afterwards.

- [ ] **Step 5: fmt, commit, push**

```bash
git add crates/api/src/rate_limit.rs crates/api/src/routes/import_export/mod.rs
git commit -m "Add the import preview route and duplicate decisions on import"
git push
```

---

### Task 6: Web schemas, API client and hooks

**Files:**
- Modify: `apps/web/src/schemas/bookmarks/bookmarks.ts:65-69, 109`
- Modify: `apps/web/src/api/api.ts:679-680` (+ schema import list)
- Modify: `apps/web/src/hooks/import-export/useImportExport.ts:1-12`

- [ ] **Step 1: Schemas**

Replace `importResultSchema` and add the preview schemas:

```ts
export const importResultSchema = z.object({
  total: z.number(),
  imported: z.number(),
  failed: z.number(),
  skipped: z.number().default(0),
  replaced: z.number().default(0),
})

export const duplicateActionSchema = z.enum(['skip', 'replace', 'keep'])

const duplicateSideSchema = z.object({
  title: z.string().nullish(),
  description: z.string().nullish(),
  tags: z.array(z.string()),
  folders: z.array(z.string()),
})

export const importPreviewSchema = z.object({
  total: z.number(),
  new: z.number(),
  duplicates: z.array(
    z.object({
      index: z.number(),
      url: z.string(),
      existing: duplicateSideSchema.extend({ id: z.string(), updated_at: z.string() }),
      incoming: duplicateSideSchema,
    })
  ),
  folders: z.object({ reused: z.number(), created: z.number() }),
})
```

Beside `export type ImportResult …`:

```ts
export type DuplicateAction = z.infer<typeof duplicateActionSchema>
export type ImportPreview = z.infer<typeof importPreviewSchema>
export type ImportDuplicate = ImportPreview['duplicates'][number]
export type DuplicateDecisions = { default: DuplicateAction; overrides: Record<number, DuplicateAction> }
```

- [ ] **Step 2: API client**

In `api.ts`, add `importPreviewSchema` to the schema import list and `type DuplicateDecisions, type ImportPreview, type ImportResult` to the type imports, then:

```ts
  importPreview: (format: string, content: string): Promise<ImportPreview> =>
    req('/import/preview', { method: 'POST', body: JSON.stringify({ format, content }) }, importPreviewSchema),

  import: (format: string, content: string, duplicates?: DuplicateDecisions): Promise<ImportResult> =>
    req(
      '/import',
      { method: 'POST', body: JSON.stringify(duplicates ? { format, content, duplicates } : { format, content }) },
      importResultSchema
    ),
```

- [ ] **Step 3: Hooks**

```ts
import type { DuplicateDecisions, ImportPreview, ImportResult } from '@/schemas'

type ImportVars = { format: string; content: string; duplicates?: DuplicateDecisions }

export function useImportBookmarks(): UseMutationResult<ImportResult, Error, ImportVars> {
  return useMutation({
    mutationFn: ({ format, content, duplicates }: ImportVars) => api.import(format, content, duplicates),
  })
}

export function useImportPreview(): UseMutationResult<ImportPreview, Error, { format: string; content: string }> {
  return useMutation({
    mutationFn: ({ format, content }) => api.importPreview(format, content),
  })
}
```

Export `useImportPreview` from `apps/web/src/hooks/import-export/index.ts` (check whether it re-exports with `export *`).

- [ ] **Step 4: Typecheck**

Run (in `apps/web`): `pnpm exec tsc -b`
Expected: clean. The onboarding page still calls `useImportBookmarks` without `duplicates`.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/schemas/bookmarks/bookmarks.ts apps/web/src/api/api.ts apps/web/src/hooks/import-export/useImportExport.ts apps/web/src/hooks/import-export/index.ts
git commit -m "Add the import preview to the web API client"
git push
```

---

### Task 7: Decision helper

**Files:**
- Create: `apps/web/src/components/import-export/duplicateDecisions.ts`
- Test: `apps/web/src/components/import-export/duplicateDecisions.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, expect, it } from 'vitest'
import { actionFor, decisionsBody } from './duplicateDecisions'

describe('duplicate decisions', () => {
  it('uses the hand-picked choice, else the default', () => {
    const picked = new Map([[3, 'replace' as const]])
    expect(actionFor(3, 'skip', picked)).toBe('replace')
    expect(actionFor(4, 'skip', picked)).toBe('skip')
  })

  it('sends only rows that differ from the default', () => {
    const picked = new Map([
      [3, 'replace' as const],
      [4, 'skip' as const],
    ])
    expect(decisionsBody('skip', picked)).toEqual({ default: 'skip', overrides: { 3: 'replace' } })
  })
})
```

- [ ] **Step 2: Run to see it fail**

Run: `pnpm exec vitest run src/components/import-export/duplicateDecisions.test.ts`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement**

```ts
import type { DuplicateAction, DuplicateDecisions } from '@/schemas'

/** A row's action: what the person picked for it, else the apply-to-all choice. */
export function actionFor(index: number, all: DuplicateAction, picked: ReadonlyMap<number, DuplicateAction>): DuplicateAction {
  return picked.get(index) ?? all
}

/** The import body's `duplicates`: the default, plus only the rows that differ from it. */
export function decisionsBody(all: DuplicateAction, picked: ReadonlyMap<number, DuplicateAction>): DuplicateDecisions {
  const overrides: Record<number, DuplicateAction> = {}
  for (const [index, action] of picked) {
    if (action !== all) overrides[index] = action
  }
  return { default: all, overrides }
}
```

- [ ] **Step 4: Run to see it pass**

Run: `pnpm exec vitest run src/components/import-export/duplicateDecisions.test.ts`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/import-export/duplicateDecisions.ts apps/web/src/components/import-export/duplicateDecisions.test.ts
git commit -m "Add the import duplicate decision helpers"
git push
```

---

### Task 8: Review drawer

**Files:**
- Create: `apps/web/src/components/import-export/ImportDuplicatesDrawer.tsx`
- Test: `apps/web/src/components/import-export/ImportDuplicatesDrawer.test.tsx`

Behavior (spec, Web app section): header count; apply-to-all `SegmentedControl` (Skip default, Replace, Import anyway) that sets every row not changed by hand; one row per duplicate with existing vs incoming and its own `SegmentedControl`; 50 rows per page with Mantine `Pagination`; **Import** calls `onConfirm(decisionsBody(all, picked))`; **Cancel** calls `onClose`; `error` shows in an `Alert` and the drawer stays open.

- [ ] **Step 1: Write the failing test**

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import type { ImportPreview } from '@/schemas'
import { ImportDuplicatesDrawer } from './ImportDuplicatesDrawer'

function dup(index: number) {
  return {
    index,
    url: `https://d${index}.example`,
    existing: { id: `b${index}`, title: `Mine ${index}`, description: null, tags: [], folders: ['Work'], updated_at: '2026-01-01T00:00:00Z' },
    incoming: { title: `Theirs ${index}`, description: null, tags: ['t'], folders: [] },
  }
}

const preview: ImportPreview = { total: 10, new: 8, duplicates: [dup(1), dup(4)], folders: { reused: 0, created: 0 } }

beforeAll(() => {
  vi.stubGlobal('ResizeObserver', class { observe(): void {} unobserve(): void {} disconnect(): void {} })
})
afterEach(cleanup)

function renderDrawer(onConfirm = vi.fn(), onClose = vi.fn()) {
  render(
    <MantineProvider env="test">
      <ImportDuplicatesDrawer opened preview={preview} pending={false} error={null} onConfirm={onConfirm} onClose={onClose} />
    </MantineProvider>
  )
  return { onConfirm, onClose }
}

describe('ImportDuplicatesDrawer', () => {
  it('says how many are already in the library and skips them by default', async () => {
    const { onConfirm } = renderDrawer()
    expect(screen.getByText('2 of 10 bookmarks are already in your library.')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Import' }))
    expect(onConfirm).toHaveBeenCalledWith({ default: 'skip', overrides: {} })
  })

  it('keeps a hand-picked row when apply-to-all changes', async () => {
    const { onConfirm } = renderDrawer()
    const row = screen.getByRole('group', { name: 'https://d4.example' })
    await userEvent.click(within(row).getByText('Import anyway'))
    await userEvent.click(within(screen.getByRole('group', { name: 'Apply to all' })).getByText('Replace'))
    await userEvent.click(screen.getByRole('button', { name: 'Import' }))
    expect(onConfirm).toHaveBeenCalledWith({ default: 'replace', overrides: { 4: 'keep' } })
  })

  it('cancel imports nothing', async () => {
    const { onConfirm, onClose } = renderDrawer()
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onClose).toHaveBeenCalled()
    expect(onConfirm).not.toHaveBeenCalled()
  })
})
```

Add `within` to the `@testing-library/react` import.

- [ ] **Step 2: Run to see it fail**

Run: `pnpm exec vitest run src/components/import-export/ImportDuplicatesDrawer.test.tsx`
Expected: FAIL, module not found.

- [ ] **Step 3: Implement**

```tsx
import { useState, type ReactElement } from 'react'
import { Alert, Badge, Button, Drawer, Group, Pagination, Paper, SegmentedControl, SimpleGrid, Stack, Text } from '@mantine/core'
import { useDrawerSize } from '@/hooks/render'
import type { DuplicateAction, DuplicateDecisions, ImportDuplicate, ImportPreview } from '@/schemas'
import { actionFor, decisionsBody } from './duplicateDecisions'

const PAGE = 50

const choices: { value: DuplicateAction; label: string }[] = [
  { value: 'skip', label: 'Skip' },
  { value: 'replace', label: 'Replace' },
  { value: 'keep', label: 'Import anyway' },
]

type Props = {
  opened: boolean
  preview: ImportPreview
  pending: boolean
  error: string | null
  onConfirm: (decisions: DuplicateDecisions) => void
  onClose: () => void
}

function Side({ label, title, description, tags, folders }: { label: string } & Pick<ImportDuplicate['incoming'], 'title' | 'description' | 'tags' | 'folders'>): ReactElement {
  return (
    <Stack gap={2}>
      <Text size="xs" c="dimmed">{label}</Text>
      <Text size="sm" fw={500}>{title || 'No title'}</Text>
      {description ? <Text size="xs" lineClamp={2}>{description}</Text> : null}
      <Text size="xs" c="dimmed">{folders.length ? folders.join(', ') : 'No folder'}</Text>
      {tags.length ? (
        <Group gap={4}>
          {tags.map((t) => <Badge key={t} size="xs" variant="light">{t}</Badge>)}
        </Group>
      ) : null}
    </Stack>
  )
}

export function ImportDuplicatesDrawer({ opened, preview, pending, error, onConfirm, onClose }: Props): ReactElement {
  const drawerSize = useDrawerSize()
  const [all, setAll] = useState<DuplicateAction>('skip')
  const [picked, setPicked] = useState<Map<number, DuplicateAction>>(new Map())
  const [page, setPage] = useState(1)
  const rows = preview.duplicates.slice((page - 1) * PAGE, page * PAGE)
  const pages = Math.ceil(preview.duplicates.length / PAGE)

  return (
    <Drawer opened={opened} onClose={onClose} title="Review duplicates" position="right" size={drawerSize} padding="lg">
      <Stack gap="md">
        <Text>{`${preview.duplicates.length} of ${preview.total} bookmarks are already in your library.`}</Text>
        <Stack gap={4} role="group" aria-label="Apply to all">
          <Text size="sm" fw={500}>Apply to all</Text>
          <SegmentedControl data={choices} value={all} onChange={(v) => setAll(v as DuplicateAction)} fullWidth />
        </Stack>
        {rows.map((d) => (
          <Paper key={d.index} withBorder p="sm" radius="md" role="group" aria-label={d.url}>
            <Stack gap="xs">
              <Text size="xs" truncate="end">{d.url}</Text>
              <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="sm">
                <Side label="In your library" {...d.existing} />
                <Side label="In the file" {...d.incoming} />
              </SimpleGrid>
              <SegmentedControl
                size="xs"
                data={choices}
                value={actionFor(d.index, all, picked)}
                onChange={(v) => setPicked((m) => new Map(m).set(d.index, v as DuplicateAction))}
              />
            </Stack>
          </Paper>
        ))}
        {pages > 1 ? <Pagination value={page} onChange={setPage} total={pages} /> : null}
        {error ? <Alert color="red">{error}</Alert> : null}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose} disabled={pending}>Cancel</Button>
          <Button onClick={() => onConfirm(decisionsBody(all, picked))} loading={pending}>Import</Button>
        </Group>
      </Stack>
    </Drawer>
  )
}
```

`Side` receives `existing` too; its extra `id`/`updated_at` props are ignored by destructuring. If TypeScript objects to the spread, pass the four fields explicitly.

- [ ] **Step 4: Run to see it pass**

Run: `pnpm exec vitest run src/components/import-export/ImportDuplicatesDrawer.test.tsx`
Expected: 3 passed. If Mantine's `SegmentedControl` labels are not clickable by text in jsdom, click the radio: `within(row).getByRole('radio', { name: 'Import anyway' })`.

- [ ] **Step 5: prettier, tsc, lint, commit**

```bash
pnpm exec prettier --write src/components/import-export/ImportDuplicatesDrawer.tsx src/components/import-export/ImportDuplicatesDrawer.test.tsx
pnpm exec tsc -b && pnpm lint
git add apps/web/src/components/import-export/ImportDuplicatesDrawer.tsx apps/web/src/components/import-export/ImportDuplicatesDrawer.test.tsx
git commit -m "Add the import duplicates review drawer"
git push
```

---

### Task 9: Wire the import panel

**Files:**
- Modify: `apps/web/src/components/import-export/ImportExportPanel.tsx:10-28` and the JSX end
- Test: `apps/web/src/components/import-export/ImportExportPanel.test.tsx` (new)

- [ ] **Step 1: Write the failing test**

```tsx
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { ImportExportPanel } from './ImportExportPanel'

const importMut = { isPending: false, mutateAsync: vi.fn() }
const previewMut = { isPending: false, mutateAsync: vi.fn() }

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useImportBookmarks: () => importMut,
  useImportPreview: () => previewMut,
  useMe: () => ({ data: { vault_unlocked: false } }),
  useExportBookmarks: () => ({ exportBookmarks: vi.fn(), isExporting: false, exportError: null, clearExportError: vi.fn() }),
}))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

beforeAll(() => {
  vi.stubGlobal('ResizeObserver', class { observe(): void {} unobserve(): void {} disconnect(): void {} })
})
beforeEach(() => {
  importMut.mutateAsync.mockReset().mockResolvedValue({ total: 1, imported: 1, failed: 0, skipped: 0, replaced: 0 })
  previewMut.mutateAsync.mockReset()
})
afterEach(cleanup)

async function pickFileAndImport(): Promise<void> {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  await userEvent.upload(input, new File(['<DL></DL>'], 'b.html', { type: 'text/html' }))
  await userEvent.click(screen.getByRole('button', { name: 'Import' }))
}

describe('ImportExportPanel import', () => {
  it('imports straight away when nothing is a duplicate', async () => {
    previewMut.mutateAsync.mockResolvedValue({ total: 1, new: 1, duplicates: [], folders: { reused: 0, created: 0 } })
    render(<MantineProvider env="test"><ImportExportPanel /></MantineProvider>)
    await pickFileAndImport()
    await waitFor(() => expect(importMut.mutateAsync).toHaveBeenCalledWith({ format: 'netscape', content: '<DL></DL>' }))
    expect(screen.queryByText('Review duplicates')).toBeNull()
  })

  it('opens the review when there are duplicates, and imports with the choices', async () => {
    previewMut.mutateAsync.mockResolvedValue({
      total: 2,
      new: 1,
      duplicates: [{ index: 0, url: 'https://d.example', existing: { id: 'b', title: 'Mine', description: null, tags: [], folders: [], updated_at: '2026-01-01T00:00:00Z' }, incoming: { title: 'Theirs', description: null, tags: [], folders: [] } }],
      folders: { reused: 0, created: 0 },
    })
    render(<MantineProvider env="test"><ImportExportPanel /></MantineProvider>)
    await pickFileAndImport()
    expect(await screen.findByText('Review duplicates')).toBeTruthy()
    expect(importMut.mutateAsync).not.toHaveBeenCalled()
    const buttons = screen.getAllByRole('button', { name: 'Import' })
    await userEvent.click(buttons[buttons.length - 1])
    await waitFor(() =>
      expect(importMut.mutateAsync).toHaveBeenCalledWith({ format: 'netscape', content: '<DL></DL>', duplicates: { default: 'skip', overrides: {} } })
    )
  })
})
```

- [ ] **Step 2: Run to see it fail**

Run: `pnpm exec vitest run src/components/import-export/ImportExportPanel.test.tsx`
Expected: FAIL (`previewMut.mutateAsync` never called; import called directly).

- [ ] **Step 3: Implement**

In `ImportExportPanel.tsx`:

```tsx
import { useExportBookmarks, useImportBookmarks, useImportPreview, useMe } from '@/hooks'
import type { DuplicateDecisions, ImportPreview, ImportResult } from '@/schemas'
import { ImportDuplicatesDrawer } from './ImportDuplicatesDrawer'

function resultMessage(res: ImportResult): string {
  const parts = [`Imported ${res.imported} of ${res.total}`]
  if (res.replaced) parts.push(`replaced ${res.replaced}`)
  if (res.skipped) parts.push(`skipped ${res.skipped}`)
  if (res.failed) parts.push(`failed ${res.failed}`)
  return parts.join(', ')
}
```

Inside the component, replace `doImport`:

```tsx
  const previewMut = useImportPreview()
  const [review, setReview] = useState<{ format: string; content: string; preview: ImportPreview } | null>(null)
  const [reviewError, setReviewError] = useState<string | null>(null)

  async function runImport(fmt: string, content: string, duplicates?: DuplicateDecisions): Promise<boolean> {
    const res = await importMut.mutateAsync(duplicates ? { format: fmt, content, duplicates } : { format: fmt, content })
    notifications.show({ message: resultMessage(res) })
    return true
  }

  async function doImport(): Promise<void> {
    if (!file || !format) return
    try {
      const content = await file.text()
      const preview = await previewMut.mutateAsync({ format, content })
      if (preview.duplicates.length === 0) {
        await runImport(format, content)
        return
      }
      setReviewError(null)
      setReview({ format, content, preview })
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  async function confirmReview(decisions: DuplicateDecisions): Promise<void> {
    if (!review) return
    try {
      await runImport(review.format, review.content, decisions)
      setReview(null)
    } catch (e) {
      // The drawer stays open with the error, so the person can retry without choosing again.
      setReviewError(e instanceof Error ? e.message : String(e))
    }
  }
```

The Import button's `loading` becomes `importMut.isPending || previewMut.isPending`. After the closing `</SimpleGrid>`, wrap the return in a fragment and add:

```tsx
      {review ? (
        <ImportDuplicatesDrawer
          opened
          preview={review.preview}
          pending={importMut.isPending}
          error={reviewError}
          onConfirm={(d) => void confirmReview(d)}
          onClose={() => setReview(null)}
        />
      ) : null}
```

`runImport` returning `boolean` is unused; make it `Promise<void>` if lint complains.

- [ ] **Step 4: Run the import-export tests, then the full web suite**

Run: `pnpm exec vitest run src/components/import-export && pnpm exec vitest run`
Expected: all pass.

- [ ] **Step 5: prettier, tsc, lint, build, commit**

```bash
pnpm exec prettier --check src/components/import-export/*.tsx src/components/import-export/*.ts
pnpm exec tsc -b && pnpm lint && pnpm build
git add apps/web/src/components/import-export/ImportExportPanel.tsx apps/web/src/components/import-export/ImportExportPanel.test.tsx
git commit -m "Review duplicates before importing in Settings"
git push
```

Then remove the build output: `rm -rf apps/web/dist`.

---

### Task 10: Docs, and close the roadmap item

**Files:**
- Modify: `docs/product/features.md` (Import / export section)
- Modify: `docs/product/roadmap.md` (remove item 17)
- Modify: `docs/CHANGELOG.md` (Unreleased → Added)
- Rebuild: `deploy/public-beta/static/docs`
- Delete: `docs/design/import-duplicates-plan.md` (this file; finished plans are pruned, as in #98)

- [ ] **Step 1: features.md** — under `## Import / export`, add:

```markdown
- Duplicates on import: before anything is written, the import checks the file against your bookmarks outside vaults by exact URL. When some are already in the library, a review lists each one beside the existing bookmark, with **Skip** (the default), **Replace** (update the existing bookmark in place from the file, keeping its shares, pins and activity) or **Import anyway**, per row or for all. The API takes the same choices: `POST /api/import/preview`, then `duplicates` on `POST /api/import`
- Folders in an imported file whose full path already exists are reused instead of created again
```

- [ ] **Step 2: roadmap.md** — delete the whole item `17. **Duplicates on import** …` up to `18.`.

- [ ] **Step 3: CHANGELOG.md** — under `## [Unreleased]` → `### Added`:

```markdown
- Import checks for bookmarks already in your library and lets you skip, replace or keep each one; folders that already exist are reused.
```

- [ ] **Step 4: Rebuild the docs site output**

Run: `cd apps/docs && pnpm build && cd ../.. && rm -rf apps/docs/build`
Expected: `published build/ → …/deploy/public-beta/static/docs`.

- [ ] **Step 5: Commit, push, open the PR**

```bash
git rm docs/design/import-duplicates-plan.md
git add docs/product/features.md docs/product/roadmap.md docs/CHANGELOG.md deploy/public-beta/static/docs
git commit -m "Document duplicates on import and close roadmap item 17"
git push
graphify update .
gh pr create --base main --title "Review duplicates on import" --body "…(Why / What changed / How it was tested / Checklist, from .github/pull_request_template.md)…"
```

---

## Self-review

- **Spec coverage:** exact URL + non-vault + live (Task 3 SQL, tests); latest of several (Task 3 test); preview writes nothing (Task 3 test); preview limits and slot (Task 5); decisions default keep (Task 1 test, Task 4 `keep_creates…`); re-check at import time (Task 4 `a_bookmark_added_after_the_preview…`); override for a non-duplicate ignored (`plan_import` only reads decisions for found URLs); quota counts creates (Task 4 `plan.creates`, Task 5); `skipped`/`replaced` in response (Task 5), no migration; Replace rules (Task 4 two tests); folder reuse by path, reused parent, siblings, exact case (Task 2); drawer header, apply-to-all over untouched rows, per-row, paging, cancel, error stays open (Tasks 8–9); onboarding unchanged (Task 6 optional `duplicates`); docs (Task 10).
- **Types:** `DuplicateAction`, `DuplicateDecisions`, `ImportPreview`, `ImportPlan`, `RowPlan`, `ImportCounts.{skipped,replaced}`, `apply_parsed_import_counted(pool, user, parsed, &plan, &mut counts)`, `plan_import`, `preview_import`, `find_own_bookmarks_by_urls`, web `actionFor`/`decisionsBody`, `useImportPreview`, `api.importPreview` are used with the same names and arguments throughout.
- **Placeholders:** the PR body in Task 10 follows the repo template; the Task 5 smoke test needs a real auth header for a test account.
