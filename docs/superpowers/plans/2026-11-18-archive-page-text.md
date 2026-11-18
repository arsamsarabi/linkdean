# Archive page text (Meili) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After each successful non-vault HTML archive, extract reader-mode page text into Postgres and Meili so library search matches page body; vault bodies never enter either store.

**Architecture:** Extend `bookmark_archives` with `page_text`. On HTML capture finish (non-vault), `linkdean_core::extract_page_text` fills `StoredCopy.page_text`; `finish_archive` persists it. `BookmarkView` carries `page_text` with `#[serde(skip)]` so every existing Meili upsert includes it without API exposure. Vault move clears DB text; Meili delete already runs for `left_search`. Delete-archive re-upserts the bookmark without body text.

**Tech Stack:** Rust (`scraper` already in workspace), Postgres, Meilisearch, existing archive runner.

**Spec:** `docs/superpowers/specs/2026-11-18-archive-page-text-design.md`

## Global Constraints

- Never extract, store, or index page text for vault bookmarks.
- Extraction failure must not fail the archive; set `page_text` NULL.
- No backfill of existing HTML archives; no Markdown download UI.
- Truncate extracted text to `PAGE_TEXT_MAX` = `100_000` chars (named constant in core).
- Prefer workspace `scraper`; do not add a readability crate unless extract quality clearly fails fixtures.
- Work on branch `docs/archive-page-text-design` (same PR #174); commit each green task and push.
- Never commit `main`. `graphify update .` after code edits. `GH_TOKEN=$(gh auth token --user arsamsarabi)` for gh.
- DB tests: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test`, `cargo test -j 2`; `archives` binary is in nextest `db-global-state`.
- Migration number: confirm next free on `origin/main` at implement time (expect `060` after `059_bookmark_archives.sql`).
- Rustfmt: CI toolchain from `.github/workflows/ci.yml` in a `rust:<ver>` container, not host rustfmt.

## Review Focus

These are covered by task tests below (not an unchecked backlog):

1. Vault HTML capture → `page_text` stays NULL even if plaintext HTML existed in memory.
2. Move into vault → DB `page_text` cleared and Meili doc deleted (no lingering body matches).
3. Delete HTML archive while bookmark remains → Meili upsert with empty `page_text`.
4. Extract empty/error → archive `done`, previous `page_text` cleared on replace.
5. Bookmark metadata edit after archive → Meili `add_or_replace` keeps `page_text` via `BookmarkView`.

## File map

| File | Responsibility |
| --- | --- |
| `migrations/060_archive_page_text.sql` (or next free) | `page_text` column on `bookmark_archives` |
| `crates/core/src/page_text.rs` (new) | `PAGE_TEXT_MAX`, `extract_page_text` |
| `crates/core/Cargo.toml`, `crates/core/src/lib.rs` | `scraper` dep + module export |
| `crates/core/src/models/mod.rs` | `BookmarkView.page_text` with `#[serde(skip)]` |
| `crates/db/src/repo/archives.rs` | `StoredCopy.page_text`; finish writes it; load map for views; force NULL when encrypted |
| `crates/db/src/repo/bookmarks.rs` | `ViewBatch` loads page texts for non-vault ids; set on views |
| `crates/db/src/repo/vault_moves.rs` | Clear `page_text` in `write_encrypted` |
| `crates/search/src/lib.rs` | `BookmarkDoc.page_text`; searchable attrs; `from_view` |
| `crates/api/src/archive_runner.rs` | Extract on non-vault HTML; Meili upsert after finish |
| `crates/api/src/routes/archives.rs` | After `delete_all`, Meili upsert bookmark |
| `crates/db/tests/archives.rs` | Lifecycle tests; update `copy()` helper |
| `docs/design/archive.md`, `docs/product/{roadmap,features}.md`, `docs/CHANGELOG.md` | Ship wording |
| `linkdean-infra/apps/docs/docs/roadmap.md` | Public roadmap if wording changes |

---

### Task 1: Extract page text in core

**Files:**
- Create: `crates/core/src/page_text.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`
- Test: unit tests in `page_text.rs`

**Interfaces:**
- Produces: `pub const PAGE_TEXT_MAX: usize = 100_000;` and `pub fn extract_page_text(html: &str) -> String` (never panics; may return empty)

- [ ] **Step 1: Write the failing test** in `crates/core/src/page_text.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn article_body_is_kept_and_chrome_is_dropped() {
        let html = r#"<html><body>
          <nav>Home About</nav>
          <article><h1>UniquePhrase</h1><p>Body words here.</p></article>
          <footer>Copyright</footer>
          <script>ignore()</script>
        </body></html>"#;
        let text = extract_page_text(html);
        assert!(text.contains("UniquePhrase"));
        assert!(text.contains("Body words"));
        assert!(!text.contains("ignore()"));
    }

    #[test]
    fn empty_or_garbage_yields_empty_or_short() {
        assert!(extract_page_text("").is_empty() || extract_page_text("").len() < 8);
        assert!(extract_page_text("<html><body></body></html>").len() < 32);
    }

    #[test]
    fn output_is_capped_at_page_text_max() {
        let para = "word ".repeat(PAGE_TEXT_MAX / 4);
        let html = format!("<html><body><article>{para}</article></body></html>");
        assert!(extract_page_text(&html).chars().count() <= PAGE_TEXT_MAX);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p linkdean-core --lib page_text -- --nocapture`  
Expected: FAIL (module / function missing)

- [ ] **Step 3: Implement**

Add `scraper.workspace = true` to `crates/core/Cargo.toml`. In `page_text.rs`: export `PAGE_TEXT_MAX` and `extract_page_text`. Algorithm (builder may refine if tests need it): parse with `scraper::Html`; prefer first `article`, else `main`, else `body`; remove `script`, `style`, `nav`, `footer`, `aside` nodes from that subtree; collect visible text; collapse whitespace; truncate by chars to `PAGE_TEXT_MAX`. Export `pub mod page_text` and `pub use page_text::{extract_page_text, PAGE_TEXT_MAX}` from `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p linkdean-core --lib page_text -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit and push**

```bash
git add crates/core/src/page_text.rs crates/core/src/lib.rs crates/core/Cargo.toml
git commit -m "$(cat <<'EOF'
Add reader-mode page text extraction in core.

EOF
)"
git push
```

---

### Task 2: Persist `page_text` and load it on bookmark views

**Files:**
- Create: `migrations/060_archive_page_text.sql` (renumber if `060` taken on `origin/main`)
- Modify: `crates/db/src/repo/archives.rs` (`StoredCopy`, `finish_archive`, new loader)
- Modify: `crates/db/src/repo/bookmarks.rs` (`ViewBatch`, `view_from_batch`)
- Modify: `crates/db/src/repo/vault_moves.rs` (`write_encrypted`)
- Modify: `crates/core/src/models/mod.rs` (`BookmarkView`)
- Modify: `crates/db/tests/archives.rs` (`copy()` helper + new tests)

**Interfaces:**
- Consumes: none from Task 1 (storage only)
- Produces: `StoredCopy { …, page_text: Option<String> }`; `finish_archive` writes `page_text` (forced NULL when `key_vault_id.is_some()` or format ≠ html); `html_page_texts(pool, ids) -> HashMap<Uuid, String>` for done HTML rows with non-null text; `BookmarkView.page_text: String` with `#[serde(skip)]` (empty default)

- [ ] **Step 1: Write failing DB tests** in `crates/db/tests/archives.rs`

Add `page_text: None` to `copy()` (and any other `StoredCopy` literals). Add tests (names exact):

1. `html_finish_stores_page_text` — request+claim HTML, `finish_archive` with `page_text: Some("UniqueBody".into())`, assert `html_page_texts` returns that string for the bookmark.
2. `encrypted_finish_never_stores_page_text` — `StoredCopy` with `key_vault_id: Some(vault_id)`, `page_text: Some("secret".into())`, finish, assert no page text loaded (or NULL).
3. `move_into_vault_clears_page_text` — store page text on plain bookmark, move into vault via existing vault test helpers / `move_to_vault`, assert `html_page_texts` empty for that id.

- [ ] **Step 2: Run tests to verify they fail**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -p linkdean-db --test archives html_finish_stores_page_text encrypted_finish_never_stores_page_text move_into_vault_clears_page_text -j 2 -- --nocapture`  
Expected: FAIL (column / field missing). Confirm no `skipping ... DATABASE_URL unset`.

- [ ] **Step 3: Migration + repo + view wiring**

Migration:

```sql
ALTER TABLE bookmark_archives
    ADD COLUMN page_text TEXT;
```

Update `finish_archive` INSERT/UPDATE to set `page_text` from `c.page_text` after applying: if `c.key_vault_id.is_some() || c.format != ArchiveFormat::Html { None } else { c.page_text.clone() }`.

Add `html_page_texts`. In `write_encrypted`, after updating the bookmark: `UPDATE bookmark_archives SET page_text = NULL WHERE bookmark_id = $1`.

On `BookmarkView`, add `#[serde(skip)] pub page_text: String`. Extend `ViewBatch` with `page_texts: HashMap<Uuid, String>` loaded via `html_page_texts` for `plain` ids only. Set `page_text` in both vault (always `String::new()`) and non-vault branches of `view_from_batch`. Fix every `BookmarkView { … }` struct literal in the crate that the compiler flags.

- [ ] **Step 4: Run tests to verify they pass**

Same command as Step 2. Expected: PASS. Also `cargo test -p linkdean-db --test archives -j 2` green (no skipping).

- [ ] **Step 5: Commit and push**

```bash
git add migrations/060_archive_page_text.sql crates/db/src/repo/archives.rs crates/db/src/repo/bookmarks.rs crates/db/src/repo/vault_moves.rs crates/core/src/models/mod.rs crates/db/tests/archives.rs
git commit -m "$(cat <<'EOF'
Store archive page_text in Postgres and clear it on vault entry.

EOF
)"
git push
```

---

### Task 3: Meili `page_text` field

**Files:**
- Modify: `crates/search/src/lib.rs`
- Test: existing unit tests in that file if any; else add a small `#[cfg(test)]` on `from_view` / searchable list

**Interfaces:**
- Consumes: `BookmarkView.page_text`
- Produces: `BookmarkDoc.page_text: String`; `ensure_index` searchable attrs include `"page_text"` after existing fields; `from_view` copies `view.page_text`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn bookmark_doc_includes_page_text_from_view() {
    // Build a minimal BookmarkView (defaults / empty vecs) with page_text = "needle".
    let doc = BookmarkDoc::from_view(&view);
    assert_eq!(doc.page_text, "needle");
}
```

(If constructing `BookmarkView` is heavy, test a private helper or make `from_view` take `page_text` — prefer using the view field so call sites stay unchanged.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p linkdean-search --lib -- --nocapture`  
Expected: FAIL on missing field / assert

- [ ] **Step 3: Implement** `page_text` on `BookmarkDoc` with `#[serde(default)]`, wire `from_view`, append `"page_text"` in `set_searchable_attributes`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p linkdean-search --lib -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit and push**

```bash
git add crates/search/src/lib.rs
git commit -m "$(cat <<'EOF'
Index archive page_text in Meilisearch.

EOF
)"
git push
```

---

### Task 4: Wire capture finish and delete-archive to extract + Meili

**Files:**
- Modify: `crates/api/src/archive_runner.rs`
- Modify: `crates/api/src/routes/archives.rs` (`delete_all`)
- Test: prefer extending `crates/db/tests/archives.rs` for persistence already done; add an API-level unit/integration only if a cheap harness exists — otherwise rely on runner unit of extract call via a focused test of the extract+StoredCopy assembly if extractable, plus manual smoke note. Minimum: compile-checked wiring + one `archive_runner` test that `extract_page_text` is applied when building non-vault HTML `StoredCopy` if that logic can be unit-tested without Chrome.

**Interfaces:**
- Consumes: `extract_page_text`, `StoredCopy.page_text`, `SearchClient::upsert_bookmark`, `bookmark_to_view`
- Produces: after successful `finish_archive` for a non-vault bookmark, Meili upsert; after `delete_all`, Meili upsert of the still-live bookmark (empty page_text)

- [ ] **Step 1: Implement capture path**

In `capture`, when pushing HTML into `copies` / `stored`: if `t.vault.is_none()`, set `page_text` from `extract_page_text` on the UTF-8 HTML string (lossy OK); if vault, leave `None`. After successful `finish_archive` and file cleanup, if non-vault: load view with empty keyring (or existing pattern) and `state.search.upsert_bookmark(&view)` with warn-on-err (same as bookmark routes).

- [ ] **Step 2: Implement delete-archive path**

In `delete_all`, after deleting files: `get_bookmark_raw` + `bookmark_to_view` + `upsert_bookmark` (warn-on-err). Skip upsert if view is vault (should not have been indexed).

- [ ] **Step 3: Verify compile + targeted tests**

Run: `cargo test -p linkdean-api --lib -j 2` and `DATABASE_URL=… cargo test -p linkdean-db --test archives -j 2`  
Expected: PASS

- [ ] **Step 4: Commit and push**

```bash
git add crates/api/src/archive_runner.rs crates/api/src/routes/archives.rs
git commit -m "$(cat <<'EOF'
Extract page text on HTML archive finish and refresh Meili on delete.

EOF
)"
git push
```

---

### Task 5: Docs and ship checklist

**Files:**
- Modify: `docs/design/archive.md`, `docs/product/roadmap.md`, `docs/product/features.md`, `docs/CHANGELOG.md`
- Modify (if public wording changes): `../linkdean-infra/apps/docs/docs/roadmap.md` in a separate commit/PR there, or same session with infra workflow

- [ ] **Step 1: Update docs**

- `archive.md`: add a short **Page text (search)** section describing this slice; keep Markdown download under Later.
- `roadmap.md` item 12: mark page-text + Meili as done; leave downloads/media/versions/etc.
- `features.md` Archive section: one bullet — search includes reader-mode text from HTML archives; vault never indexed; existing copies gain it on next archive.
- `CHANGELOG.md` under `## [Unreleased]` → `### Added`: one line for page-content search from archives.

- [ ] **Step 2: Fmt + graphify**

Run rustfmt in CI’s `rust:<ver>` container on touched Rust; `graphify update .` from repo root.

- [ ] **Step 3: Commit and push**

```bash
git add docs/design/archive.md docs/product/roadmap.md docs/product/features.md docs/CHANGELOG.md
git commit -m "$(cat <<'EOF'
Document archive page-text search as shipped.

EOF
)"
git push
```

- [ ] **Step 4: Update PR body** on #174 to note plan + implementation (or open follow-up if this PR stayed docs-only until tasks land — keep one PR as requested).

---

## Self-review notes (author)

- Spec coverage: extract, Postgres, Meili, vault clear, delete-archive Meili refresh, no backfill, docs — all tasked.
- `BookmarkView.page_text` + `#[serde(skip)]` keeps every existing upsert path correct without a call-site migration.
- Review Focus items mapped to Task 2 tests (1–2, 4 via finish NULL), Task 4 delete upsert (3), and ViewBatch+search (5).
