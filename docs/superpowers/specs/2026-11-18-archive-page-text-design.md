# Archive page text (Meili index) — design

Date: 2026-11-18  
Roadmap: item **12** (phase 2 slice)  
Status: approved in conversation; awaiting implementation plan

## Intent

People should find bookmarks by words that appear in the archived page, not only in title, URL, description, notes, or tags. The HTML snapshot from archive slice 1 is the source. Vault privacy must not weaken: vault bodies never land in Postgres plaintext or in Meilisearch.

## Goals

- After a successful **non-vault HTML** archive (new or replace), extract reader-mode page text, store it, and index it in Meili under a searchable `page_text` field.
- Moving a bookmark **into** a vault clears stored page text and removes the Meili document so prior content cannot keep matching.
- Vault bookmarks are never extracted, never stored as page text, and never upserted to Meili (existing skip stays; defense in depth).

## Non-goals

- Reader-mode Markdown download UI
- Full-export zip including archives
- Backfill / re-extract for HTML archives that already exist
- PDF or `file` OCR / text extraction
- Search-result snippets or highlights in the web UI
- AI summaries or embeddings (roadmap **16**)

## Anti-goals

- Vault page body in Postgres or Meili in any form
- Failing an archive because extraction failed
- Reopening archive slice-1 decisions (capture, formats, CSP, quotas)

## Constraints

- Meili updates today use full-document `add_or_replace`. Any field not present on the upserted doc is lost — so page text must live in Postgres and ride along on every non-vault bookmark upsert.
- Archive capture already has plaintext HTML bytes **before** vault encryption in `archive_runner`; extract only on the non-vault path.
- Vault move already returns `left_search` / `joined_search` and the API deletes Meili docs for ids that left search. Page-text clearing must happen in the DB move (same transaction as becoming vault), not only in Meili.
- No backfill: existing done HTML rows stay with `page_text` NULL until the person archives again.
- Prefer existing workspace deps (`scraper` et al.) for extraction; add a crate only if reader-mode quality needs it. Builder chooses the algorithm within that rule.

## Approaches considered

1. **Postgres column + Meili field (chosen).** Extract on HTML finish → store on the HTML `bookmark_archives` row → include in the search document path. Survives `add_or_replace`; supports later Markdown download without redoing storage.
2. **Meili-only patch.** Less schema; fragile under full replace and admin reindex.
3. **Re-extract on every index.** No stored text; every bookmark edit pays disk/CPU; fights the “no backfill” spirit on reindex.

## Design

### Data model

- Migration: add nullable `page_text TEXT` to `bookmark_archives`.
- Meaningful only for `format = 'html'` and non-vault copies. PDF/file rows stay NULL.
- Truncate extracted text to about **100 000** characters before store (builder may tune slightly; keep a named constant).
- Do not expose `page_text` on public bookmark JSON unless a later download slice needs it. Internal: available when building `BookmarkDoc` (join or batch-load from HTML archive rows).

### Extraction

- Pure function in `linkdean_core` (e.g. `extract_page_text(html: &str) -> String`): reader-mode main content, strip scripts/nav chrome as far as the chosen approach allows, normalise whitespace, truncate.
- Unit-tested on fixture HTML (article-like page → body text present; empty/boilerplate-heavy → empty or short string OK).
- Runs in the API archive worker on plaintext HTML bytes when the bookmark is **not** in a vault. Vault captures skip extract entirely.

### Write path (archive finish)

1. Non-vault HTML copy succeeds → extract → pass `page_text` into `finish_archive` (or a follow-up update in the same logical finish) for the HTML row.
2. Replace: new extract overwrites previous `page_text`.
3. Extract empty or error → archive still `done`; set `page_text` to NULL (clear any previous text on replace).
4. Vault HTML → never write `page_text`.
5. After finish for a non-vault bookmark, upsert Meili with the bookmark view that includes `page_text` (same warn-on-failure pattern as other upserts).

### Search

- Extend `BookmarkDoc` with `page_text: String` (default empty).
- `ensure_index`: add `page_text` to searchable attributes (keep existing fields).
- `upsert_bookmark` / batch: still skip vault and trash; `page_text` comes from the view/doc builder.
- No API/UI change for hit shape this slice — matching on body still returns the same `SearchHit` card fields.

### Vault and lifecycle

| Event | `page_text` (HTML row) | Meili |
| --- | --- | --- |
| Non-vault HTML archive done | Set / replace / NULL if empty | Upsert (includes `page_text`) |
| Vault capture | Leave NULL; do not extract | No upsert (vault) |
| Move into vault (`left_search`) | **Clear in DB transaction** | `delete_bookmark` (existing) |
| Move out of vault (`joined_search`) | Stay NULL until next HTML archive | Upsert without body until then |
| Delete HTML archive (bookmark kept) | Cleared with HTML row | **Immediate** Meili upsert without `page_text` so body terms stop matching (builder call: same spirit as vault — source gone ⇒ search must not keep the body) |
| Purge / permanently delete bookmark | Cascade | `delete_bookmark` as today |
| Trash | Unchanged in DB | Delete / restore as today (restore upsert includes `page_text` if present) |

### Failures

- Extraction must not fail the archive. Log at debug/warn if useful; user sees a normal successful archive.
- Meili upsert failure after DB write: warn only; DB remains source of truth for the next upsert.

### Testing

- `core`: extract fixtures.
- `db`: HTML finish stores text; vault finish does not; move into vault clears text; archive delete drops the row.
- API / search: after delete-HTML-archive, Meili doc has no body; vault upsert no-ops; searchable attrs include `page_text` (unit or thin integration as the harness allows).
- No new Playwright requirement for this slice.

### Docs when shipping

- `docs/design/archive.md`: move this slice out of vague “Later” into a short “Page text (search)” section; keep Markdown download under Later/Downloads.
- `docs/product/roadmap.md`: mark page-text + Meili as shipped under item 12; leave download / media / versions.
- `docs/product/features.md` + `docs/CHANGELOG.md` under Unreleased.
- Public copy in `linkdean-infra/apps/docs/docs/roadmap.md` if the public wording changes.

## Left to the builder

- Exact extract implementation and whether a new crate is justified.
- Whether `page_text` is passed through `StoredCopy` / `finish_archive` or updated in a dedicated call immediately after finish.
- How `BookmarkView` or the Meili upsert path loads `page_text` (join in view batch vs side map) — must be correct for single and batch upserts, including import.
- Exact truncation constant and any Meili document-size guard.
- Migration number: next free on `main` at implement time.

## Success

- New non-vault HTML archives make page body terms findable in library search.
- Vault bookmarks never contribute body text; move-into-vault stops prior body matches.
- Existing HTML archives without re-archive do not suddenly gain body search (accepted for this slice).
- Deleting the HTML archive stops body matches even though the bookmark remains.
