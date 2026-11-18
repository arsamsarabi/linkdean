# Archive Markdown download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let anyone who can download an HTML archive also download a readable Markdown version (headings + paragraphs) converted on the fly from that copy.

**Architecture:** Extend `GET /bookmarks/{id}/archive/html` with `as=md`. After the existing load/decrypt path, convert plaintext HTML via a new `linkdean-core` function (`html_to_reader_markdown`), return UTF-8 Markdown with attachment headers. UI adds a Markdown download next to Download on available HTML rows (drawer + viewer). No new storage or archive format row.

**Tech Stack:** Rust (`scraper` already in workspace), axum archive routes, React/Mantine archive UI.

**Spec:** `docs/superpowers/specs/2026-11-18-archive-markdown-download-design.md`

## Global Constraints

- Prefer workspace `scraper`; do not add a Markdown/readability crate unless fixtures clearly fail.
- Cap Markdown output by Unicode scalars at `PAGE_TEXT_MAX` (`100_000`).
- Never store Markdown on disk or in Postgres; vault plaintext only in memory after existing decrypt.
- `as=md` only for path format `html`; always `Content-Disposition: attachment` with `<site>-<date>.md`.
- Empty conversion → `Error::BadRequest("No readable text in this copy.")` (not 500; do not touch the archive row).
- No export zip, media, versions, tiers, import auto-archive, or hide-when-disabled work in this plan.
- Branch from fresh remote: `git fetch origin && git switch -c feat/archive-markdown-download origin/main`. Never commit `main`. Follow `.claude/skills/linkdean-repo-workflow/SKILL.md` for commit/push/PR on that branch.
- After code edits: `graphify update .`. Rustfmt via CI toolchain container from `.github/workflows/ci.yml`, not host rustfmt.
- Include the design spec in the first commit on the branch (currently untracked on the working tree).

## Review Focus

Covered by task tests below:

1. `as=md` on `pdf` / `file` → client error (not conversion).
2. Empty / chrome-only HTML → `BadRequest` with the exact message; archive unchanged.
3. Markdown response always has attachment disposition and `.md` filename (never inline).
4. Conversion keeps headings/paragraphs and drops `nav`/`footer`/`aside`/`script`/`style`.
5. Output longer than `PAGE_TEXT_MAX` is truncated (multibyte-safe).

Vault locked continues to use the existing decrypt path (`VaultLocked`); no new vault storage paths.

## File map

| File | Responsibility |
| --- | --- |
| `crates/core/src/page_text.rs` (or sibling `page_markdown.rs` if clearer) | `html_to_reader_markdown(&str) -> String` |
| `crates/core/src/lib.rs` | Export module if new file |
| `crates/api/src/routes/archives.rs` | `CopyQuery.as`, convert branch, Markdown headers / naming |
| `apps/web/src/lib/archive/archive.ts` | `archiveFileUrl` Markdown query (`as=md`) |
| `apps/web/src/lib/archive/archive.test.ts` | URL helper assertions |
| `apps/web/src/components/archive/ArchiveSection.tsx` | Markdown download link on available HTML |
| `apps/web/src/pages/archive/ArchivePage.tsx` | Markdown download when current format is HTML |
| Docs in Task 4 | features, roadmap, archive design, CHANGELOG |

---

### Task 1: Core HTML → reader Markdown

**Files:**
- Modify: `crates/core/src/page_text.rs` (preferred: keep next to extract; or create `crates/core/src/page_markdown.rs` + `lib.rs` export if the file would get hard to read)
- Test: same file `#[cfg(test)]`

**Interfaces:**
- Consumes: `PAGE_TEXT_MAX`, same root selection rules as `extract_page_text` (`article` → `main` → `body`)
- Produces: `pub fn html_to_reader_markdown(html: &str) -> String`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn headings_and_paragraphs_become_markdown() {
    let html = r#"<html><body>
      <article><h1>Title</h1><p>First para.</p><h2>Sub</h2><p>Second.</p></article>
    </body></html>"#;
    let md = html_to_reader_markdown(html);
    assert!(md.contains("# Title"));
    assert!(md.contains("## Sub"));
    assert!(md.contains("First para."));
    assert!(md.contains("Second."));
}

#[test]
fn chrome_is_dropped() {
    let html = r#"<html><body>
      <nav>Nav noise</nav>
      <article><h1>Keep</h1><p>Body</p></article>
      <footer>Footer noise</footer>
      <script>ignore()</script>
    </body></html>"#;
    let md = html_to_reader_markdown(html);
    assert!(md.contains("# Keep"));
    assert!(md.contains("Body"));
    assert!(!md.contains("Nav noise"));
    assert!(!md.contains("Footer noise"));
    assert!(!md.contains("ignore"));
}

#[test]
fn output_is_capped_at_page_text_max_multibyte() {
    let unit = "字";
    let many = unit.repeat(PAGE_TEXT_MAX + 50);
    let html = format!("<article><p>{many}</p></article>");
    let md = html_to_reader_markdown(&html);
    assert!(md.chars().count() <= PAGE_TEXT_MAX);
}

#[test]
fn empty_or_garbage_yields_empty() {
    assert!(html_to_reader_markdown("").is_empty());
    assert!(html_to_reader_markdown("<html><body><nav>x</nav></body></html>").is_empty()
        || html_to_reader_markdown("<html><body></body></html>").is_empty());
}
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p linkdean-core html_to_reader_markdown headings_and_paragraphs -- --nocapture`  
Expected: FAIL (function missing)

- [ ] **Step 3: Implement `html_to_reader_markdown`**

Reuse `select_root` / chrome skip. Emit `#`…`######` for `h1`–`h6`, blank-line-separated paragraphs for other text blocks. List items may become plain paragraphs this slice. Cap with `.chars().take(PAGE_TEXT_MAX)`. Never panic.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p linkdean-core page_text -- --nocapture` (or the new module’s tests)  
Expected: PASS

- [ ] **Step 5: Commit and push**

```bash
git add crates/core/src/page_text.rs crates/core/src/page_markdown.rs crates/core/src/lib.rs \
  docs/superpowers/specs/2026-11-18-archive-markdown-download-design.md \
  docs/superpowers/plans/2026-11-18-archive-markdown-download.md
git commit -m "$(cat <<'EOF'
Add reader-mode HTML to Markdown conversion for archive downloads.

EOF
)"
git push -u origin HEAD
```

---

### Task 2: API `as=md` on HTML copy

**Files:**
- Modify: `crates/api/src/routes/archives.rs` (`CopyQuery`, `get_copy`, `copy_headers` / `download_name` or a small Markdown-header helper)

**Interfaces:**
- Consumes: `linkdean_core::page_text::html_to_reader_markdown` (or `page_markdown::…`)
- Produces: `GET …/archive/html?as=md` → Markdown bytes + attachment; invalid `as` / non-html → `BadRequest`; empty MD → `BadRequest("No readable text in this copy.")`

- [ ] **Step 1: Write failing unit tests** in `archives.rs` `#[cfg(test)]`

Pin:

- `as=md` headers: `Content-Type: text/markdown; charset=utf-8`, disposition `attachment; filename="example.com-2026-11-03.md"` (reuse `day()` helper).
- Unknown `as` value rejected by a small pure helper (e.g. `parse_as_param`) returning `Err`.
- `as=md` with path format `pdf` rejected by that helper / guard.

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p linkdean-api --lib archives::tests -- --nocapture`  
Expected: FAIL

- [ ] **Step 3: Implement**

- Extend `CopyQuery` with `#[serde(rename = "as")] as_: Option<String>` (or `as_format`).
- After decrypt in `get_copy`: if `as_` is `Some("md")` and format is `Html`, convert; if empty → `Error::BadRequest("No readable text in this copy.".into())`; replace response body with `md.into_bytes()`; set Markdown headers (always attachment). If `as_` is `Some(other)` → `BadRequest`. If `as_=md` and format ≠ `Html` → `BadRequest`.
- Extend naming so Markdown uses `.md` (optional ext override on `download_name` is fine).

- [ ] **Step 4: Run — expect PASS**

Run: `cargo test -p linkdean-api --lib archives::tests -j 2`  
Expected: PASS

- [ ] **Step 5: Commit and push**

```bash
git add crates/api/src/routes/archives.rs
git commit -m "$(cat <<'EOF'
Serve reader Markdown from HTML archives via as=md.

EOF
)"
git push
```

---

### Task 3: Web Markdown download affordance

**Files:**
- Modify: `apps/web/src/lib/archive/archive.ts`, `archive.test.ts`
- Modify: `apps/web/src/components/archive/ArchiveSection.tsx`
- Modify: `apps/web/src/pages/archive/ArchivePage.tsx`

**Interfaces:**
- Consumes: API query `as=md`
- Produces: `archiveFileUrl(id, format, download?, asMarkdown?)` or dedicated `archiveMarkdownUrl(id)` returning `/api/bookmarks/{id}/archive/html?download=1&as=md`

- [ ] **Step 1: Failing helper test**

```ts
it('builds the Markdown download address for HTML', () => {
  expect(archiveMarkdownUrl('b1')).toBe('/api/bookmarks/b1/archive/html?download=1&as=md')
  // or whatever exact helper signature you choose
})
```

- [ ] **Step 2: Implement helper + UI**

- Show **Markdown** only when `a.format === 'html' && a.available` in `ArchiveSection`.
- On `ArchivePage`, when the current stored format is `html`, add a Markdown download control next to Download (toolbar and any fallback download path that applies to HTML).
- Label: `Markdown`. Same `IconDownload` or text-only link is fine; keep the existing Download for the raw HTML.

- [ ] **Step 3: Run**

Run: `cd apps/web && pnpm exec vitest run src/lib/archive/archive.test.ts`  
Expected: PASS

- [ ] **Step 4: Commit and push**

```bash
git add apps/web/src/lib/archive apps/web/src/components/archive/ArchiveSection.tsx \
  apps/web/src/pages/archive/ArchivePage.tsx
git commit -m "$(cat <<'EOF'
Offer Markdown download next to HTML archive copies.

EOF
)"
git push
```

---

### Task 4: Docs and ship checklist

**Files:** `docs/design/archive.md`, `docs/product/features.md`, `docs/product/roadmap.md`, `docs/CHANGELOG.md`

- [ ] **Step 1: Update wording**

- `archive.md`: move Markdown download out of Later into a short “Markdown download” note (on-demand from HTML copy; vault while unlocked).
- `features.md` Archive: one bullet for Markdown download from HTML copies.
- `roadmap.md` item 12: remove or strike the “reader-mode Markdown per bookmark” part of the Downloads bullet; leave export zip and the rest.
- `CHANGELOG.md` Unreleased: one line under Added.

- [ ] **Step 2: rustfmt in CI container; `graphify update .`**

- [ ] **Step 3: Commit, push, open or refresh PR**

```bash
git add docs/design/archive.md docs/product/features.md docs/product/roadmap.md docs/CHANGELOG.md
git commit -m "$(cat <<'EOF'
Document archive Markdown download.

EOF
)"
git push
# open PR if none yet (linkdean-repo-workflow)
```

---

## Self-review notes (author)

- Spec coverage: on-demand convert, vault in-memory only, UI on HTML rows, naming, empty → BadRequest, no new format row, docs — all tasked.
- Empty `as` / missing HTML file still use existing `NotFound` paths; only empty conversion uses the new BadRequest message.
- No full HTTP vault integration test harness for `get_copy`; vault locked remains the shared decrypt branch.
