# Archive page text (Meili) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After each successful non-vault HTML archive, extract reader-mode page text into Postgres and Meili so library search matches page body; vault bodies never enter either store.

**Architecture:** Extend `bookmark_archives` with `page_text`. On HTML capture finish, extract into `StoredCopy.page_text`; `finish_archive` persists it only when the bookmark row is still non-vault (`SELECT is_vault … FOR SHARE`). `ViewBatch` does **not** load page text. An API `search_index` helper loads `html_page_texts` only for Meili upserts and replaces every direct `state.search.upsert_bookmark(s)` call site. Vault move clears DB text; delete-archive and every non-vault finish re-upsert via the helper. Search retrieves only `id`.

**Tech Stack:** Rust (`scraper` already in workspace), Postgres, Meilisearch, existing archive runner.

**Spec:** `docs/superpowers/specs/2026-11-18-archive-page-text-design.md`

## Global Constraints

- Never extract, store, or index page text for vault bookmarks (including finish-after-move races).
- Do not load `page_text` in `ViewBatch` / ordinary list paths.
- Extraction failure must not fail the archive; set `page_text` NULL.
- No backfill of existing HTML archives; no Markdown download UI.
- Truncate by **chars** to `PAGE_TEXT_MAX` = `100_000`.
- Prefer workspace `scraper`; do not add a readability crate unless extract quality clearly fails fixtures.
- Shared-category invitees may match on the owner’s page body — accepted; do not special-case.
- Work on branch `docs/archive-page-text-design` (same PR #174); commit each green task and push.
- Never commit `main`. `graphify update .` after code edits. `GH_TOKEN=$(gh auth token --user arsamsarabi)` for gh.
- DB tests: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test`, `cargo test -j 2`; `archives` binary is in nextest `db-global-state`.
- Migration number: confirm next free on `origin/main` at implement time (expect `060` after `059_bookmark_archives.sql`).
- Rustfmt: CI toolchain from `.github/workflows/ci.yml` in a `rust:<ver>` container, not host rustfmt.

## Review Focus

Covered by task tests below:

1. Finish while bookmark is already vault (SQL `is_vault=true`, `key_vault_id: None`, `page_text: Some(..)`) → stored NULL.
2. Move into vault → DB `page_text` cleared; Meili delete via existing `left_search`.
3. `delete_archives` → no page text left for that bookmark.
4. `file` finish after HTML with text → HTML gone / `html_page_texts` empty.
5. Metadata edit / admin reindex via helper keeps or restores `page_text` on Meili docs (no silent wipe).
6. Search hit path does not retrieve `page_text`.

## File map

| File | Responsibility |
| --- | --- |
| `migrations/060_archive_page_text.sql` (or next free) | `page_text` column |
| `crates/core/src/page_text.rs` | `PAGE_TEXT_MAX`, `extract_page_text` |
| `crates/core/Cargo.toml`, `crates/core/src/lib.rs` | `scraper` + export |
| `crates/core/src/models/mod.rs` | `BookmarkView.page_text` with `#[serde(skip)]` (default empty; filled only by helper) |
| `crates/db/src/repo/archives.rs` | `StoredCopy.page_text`; finish + `is_vault FOR SHARE`; `html_page_texts` |
| `crates/db/src/repo/vault_moves.rs` | Clear `page_text` in `write_encrypted` |
| `crates/search/src/lib.rs` | `BookmarkDoc.page_text`; searchable attrs; search retrieves `id` only |
| `crates/api/src/search_index.rs` (new) | `upsert` / `upsert_many` helpers that load page texts |
| `crates/api/src/routes/{bookmarks,categories,trash,vault,import_export,admin,search}.rs` | Call helper instead of `state.search.upsert_*` |
| `crates/api/src/archive_runner.rs` | Extract; helper upsert after **every** non-vault finish; Known issue note for unencrypted file race |
| `crates/api/src/routes/archives.rs` | `delete_all` → helper upsert |
| `crates/db/tests/archives.rs` | Lifecycle tests |
| Docs listed in Task 6 | Ship wording |

---

### Task 1: Extract page text in core

**Files:**
- Create: `crates/core/src/page_text.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`

**Interfaces:**
- Produces: `pub const PAGE_TEXT_MAX: usize = 100_000;` and `pub fn extract_page_text(html: &str) -> String`

- [ ] **Step 1: Write the failing tests** in `page_text.rs`

```rust
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
    assert!(!text.contains("Home About"));
    assert!(!text.contains("Copyright"));
}

#[test]
fn empty_or_garbage_yields_empty_or_short() {
    assert!(extract_page_text("").len() < 8);
    assert!(extract_page_text("<html><body></body></html>").len() < 32);
}

#[test]
fn output_is_capped_at_page_text_max_ascii() {
    let para = "word ".repeat(PAGE_TEXT_MAX / 4);
    let html = format!("<html><body><article>{para}</article></body></html>");
    assert!(extract_page_text(&html).chars().count() <= PAGE_TEXT_MAX);
}

#[test]
fn output_is_capped_at_page_text_max_multibyte() {
    let para = "é".repeat(PAGE_TEXT_MAX + 10);
    let html = format!("<html><body><article>{para}</article></body></html>");
    let text = extract_page_text(&html);
    assert!(text.chars().count() <= PAGE_TEXT_MAX);
    assert!(!text.is_empty());
}
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p linkdean-core --lib page_text -- --nocapture`

- [ ] **Step 3: Implement**

Add `scraper.workspace = true` to `crates/core/Cargo.toml`. Root selection: if exactly one `article`, use it; else `main` if present; else `body`. Strip `script`/`style`/`nav`/`footer`/`aside`; collect text; collapse whitespace; truncate by `chars().take(PAGE_TEXT_MAX)`. Export from `lib.rs`.

- [ ] **Step 4: Run tests — expect PASS**

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

### Task 2: Persist `page_text` (no ViewBatch load)

**Files:**
- Create: `migrations/060_archive_page_text.sql` (renumber if taken)
- Modify: `crates/db/src/repo/archives.rs`, `crates/db/src/repo/vault_moves.rs`
- Modify: `crates/core/src/models/mod.rs` — add `#[serde(skip)] pub page_text: String` default `String::new()` on `BookmarkView`; set `String::new()` at every existing struct literal (compiler-driven). **Do not** load it in `ViewBatch`.
- Modify: `crates/db/tests/archives.rs`

**Interfaces:**
- Produces: `StoredCopy.page_text: Option<String>`; `finish_archive` gates storage; `pub async fn html_page_texts(pool, ids) -> Result<HashMap<Uuid, String>>`

- [ ] **Step 1: Write failing DB tests** (exact names)

Update `copy()` with `page_text: None`.

1. `html_finish_stores_page_text`
2. `encrypted_finish_never_stores_page_text` — `key_vault_id: Some(..)`, `page_text: Some("secret")` → NULL
3. `finish_after_bookmark_is_vault_stores_no_page_text` — after claim/running, `UPDATE bookmarks SET is_vault = TRUE, …` as needed for the schema, then `finish_archive` with `key_vault_id: None, page_text: Some("late")` → NULL
4. `move_into_vault_clears_page_text`
5. `file_finish_clears_html_page_text` — HTML with text, then finish a `file` copy → `html_page_texts` empty
6. `delete_archives_clears_page_text` — HTML with text, `delete_archives` → empty

- [ ] **Step 2: Run tests — expect FAIL**

Run: `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -p linkdean-db --test archives html_finish_stores_page_text encrypted_finish_never_stores_page_text finish_after_bookmark_is_vault_stores_no_page_text move_into_vault_clears_page_text file_finish_clears_html_page_text delete_archives_clears_page_text -j 2 -- --nocapture`  
Confirm no `skipping ... DATABASE_URL unset`.

- [ ] **Step 3: Implement**

Migration: `ALTER TABLE bookmark_archives ADD COLUMN page_text TEXT;`

In `finish_archive`, after beginning the transaction and locking archive rows:

```rust
let is_vault: bool = sqlx::query_scalar(
    "SELECT is_vault FROM bookmarks WHERE id = $1 FOR SHARE",
)
.bind(bookmark_id)
.fetch_one(&mut *tx)
.await
.map_err(db_err)?;
```

When writing each copy’s `page_text` bind: `None` unless `!is_vault && c.key_vault_id.is_none() && c.format == ArchiveFormat::Html`.

Add `html_page_texts`. In `write_encrypted`: `UPDATE bookmark_archives SET page_text = NULL WHERE bookmark_id = $1`.

- [ ] **Step 4: Run tests — expect PASS** (full `--test archives` too)

- [ ] **Step 5: Commit and push**

```bash
git add migrations/060_archive_page_text.sql crates/db/src/repo/archives.rs crates/db/src/repo/vault_moves.rs crates/core/src/models/mod.rs crates/db/tests/archives.rs
git commit -m "$(cat <<'EOF'
Store archive page_text with vault-safe finish locking.

EOF
)"
git push
```

---

### Task 3: Meili field + retrieve only `id`

**Files:**
- Modify: `crates/search/src/lib.rs`

**Interfaces:**
- Consumes: `view.page_text` when building docs
- Produces: searchable `page_text`; `search` uses attributes-to-retrieve limited to `id`

- [ ] **Step 1: Write failing tests**

- `bookmark_doc_includes_page_text_from_view` — `page_text = "needle"`
- `search_query_retrieves_only_id` — assert the search builder configuration / a small helper that returns the retrieve list equals `["id"]` (extract a `fn search_attributes_to_retrieve() -> &'static [&'static str] { &["id"] }` if the SDK makes direct assertion awkward)

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement** `BookmarkDoc.page_text`, append `"page_text"` last in searchable attrs, wire `from_view`, add `.with_attributes_to_retrieve(Selectors::Some(&["id"]))` (or SDK equivalent) on the search query.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit and push**

```bash
git add crates/search/src/lib.rs
git commit -m "$(cat <<'EOF'
Index page_text in Meili and retrieve only hit ids.

EOF
)"
git push
```

---

### Task 4: `search_index` helper and call-site migration

**Files:**
- Create: `crates/api/src/search_index.rs`
- Modify: `crates/api/src/main.rs` or `routes` mod tree to `mod search_index`
- Modify every direct upsert site: `routes/bookmarks/mod.rs`, `categories/mod.rs`, `trash/mod.rs`, `vault/mod.rs`, `import_export/mod.rs`, `admin/mod.rs`, `search/mod.rs`
- Test: unit test or `#[test]` that greps/fails if `state.search.upsert_bookmark` / `upsert_bookmarks` appear outside `search_index.rs` (string scan of `crates/api/src/**/*.rs` in a test under `search_index`, or a tiny `tests/search_index_call_sites.rs`). Admin reindex path must call the helper — add a focused test or assert in the grep allowlist that `admin` uses `search_index::`.

**Interfaces:**
- Produces:
  - `pub async fn upsert(state: &AppState, view: &BookmarkView)`
  - `pub async fn upsert_many(state: &AppState, views: &[BookmarkView])`
  - Both: skip vault/trash the same way `SearchClient` does; for remaining ids load `html_page_texts`, set `page_text` on clones (or rebuild docs), then `state.search.upsert_*`. Warn-on-err matching today’s pattern.

- [ ] **Step 1: Write the call-site guard test** (fails while direct calls remain)

- [ ] **Step 2: Implement helper + replace all call sites** (including admin reindex). Keep `SearchClient` methods for the helper only.

- [ ] **Step 3: Run** `cargo test -p linkdean-api --lib -j 2` — PASS; guard test PASS

- [ ] **Step 4: Commit and push**

```bash
git add crates/api/src/search_index.rs crates/api/src/main.rs crates/api/src/routes
git commit -m "$(cat <<'EOF'
Route Meili upserts through a helper that loads page_text.

EOF
)"
git push
```

---

### Task 5: Wire archive finish + delete-archive

**Files:**
- Modify: `crates/api/src/archive_runner.rs`, `crates/api/src/routes/archives.rs`

**Interfaces:**
- Consumes: `extract_page_text`, `search_index::upsert`

- [ ] **Step 1: Capture path**

On non-vault HTML bytes, set `StoredCopy.page_text = Some(extract_page_text(...))` (empty → still Some empty or None — finish stores NULL for empty). Vault: `None`.

After **every** successful non-vault `finish_archive` (any formats, including PDF-only and `file`), call `search_index::upsert` so HTML replacement / deletion cannot leave stale Meili body text.

Add `Known issue:` above the finish/rekey boundary: finish-vs-vault-move can leave the HTML **file** unencrypted (slice-1); suggested fix: rekey or check vault after finish / hold bookmark lock across capture+finish.

- [ ] **Step 2: Delete-archive path**

After `delete_archives` + file removal: load view, `search_index::upsert` (empty page_text). DB already covered by `delete_archives_clears_page_text`.

- [ ] **Step 3: Run** `cargo test -p linkdean-api --lib -j 2` and full `archives` DB tests — PASS

- [ ] **Step 4: Commit and push**

```bash
git add crates/api/src/archive_runner.rs crates/api/src/routes/archives.rs
git commit -m "$(cat <<'EOF'
Extract page text on archive finish and refresh Meili on delete.

EOF
)"
git push
```

---

### Task 6: Docs and ship checklist

**Files:** `docs/design/archive.md`, `docs/product/roadmap.md`, `docs/product/features.md`, `docs/CHANGELOG.md`; infra public roadmap if needed

- [ ] **Step 1:** Document page-text search, vault rules, shared-category body matches, no backfill; mark roadmap item; features + changelog Unreleased.
- [ ] **Step 2:** rustfmt in CI container; `graphify update .`
- [ ] **Step 3:** Commit, push, refresh PR #174 body

---

## Self-review notes (author)

- PR #174 review blockers addressed: `is_vault FOR SHARE` on finish; no ViewBatch load; search_index helper + call-site guard; search retrieve `id`; upsert after every non-vault finish; required DB tests for vault race, file replace, delete; extract chrome/multibyte/root rules; shared-category accepted in spec.
