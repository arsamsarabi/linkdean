# Archive page text (Meili index) — design

Date: 2026-11-18  
Roadmap: item **12** (phase 2 slice)  
Status: approved in conversation; revised after PR #174 review

## Intent

People should find bookmarks by words that appear in the archived page, not only in title, URL, description, notes, or tags. The HTML snapshot from archive slice 1 is the source. Vault privacy must not weaken: vault bodies never land in Postgres plaintext or in Meilisearch.

## Goals

- After a successful **non-vault HTML** archive (new or replace), extract reader-mode page text, store it, and index it in Meili under a searchable `page_text` field.
- Moving a bookmark **into** a vault clears stored page text and removes the Meili document so prior content cannot keep matching.
- Vault bookmarks are never extracted, never stored as page text, and never upserted to Meili (existing skip stays; defense in depth).
- Removing the HTML archive (delete archive, or a `file` finish that replaces HTML) stops body matches even though the bookmark remains.

## Non-goals

- Reader-mode Markdown download UI
- Full-export zip including archives
- Backfill / re-extract for HTML archives that already exist
- PDF or `file` OCR / text extraction
- Search-result snippets or highlights in the web UI
- AI summaries or embeddings (roadmap **16**)

## Anti-goals

- Vault page body in Postgres or Meili in any form (including races where capture finishes after a vault move)
- Loading up to 100k chars of `page_text` on ordinary library / dashboard / search card queries
- Returning full `page_text` in Meili search hit payloads
- Failing an archive because extraction failed
- Reopening archive slice-1 decisions (capture, formats, CSP, quotas)

## Constraints

- Meili updates today use full-document `add_or_replace`. Any field not present on the upserted doc is lost — so page text must live in Postgres and be attached on **every** non-vault Meili upsert (metadata edit, import, admin reindex, archive finish, delete-archive refresh).
- Ordinary bookmark list/view paths must **not** load `page_text` from Postgres. Only the Meili upsert helper loads it.
- Archive capture already has plaintext HTML bytes **before** vault encryption in `archive_runner`; extract only when the capture target is non-vault. `finish_archive` must also refuse to store `page_text` when the bookmark row is already vault (`SELECT is_vault … FOR SHARE` in the finish transaction), so a move that commits first cannot be overwritten by a late finish.
- Vault move already returns `left_search` / `joined_search` and the API deletes Meili docs for ids that left search. Page-text clearing must happen in the DB move (same transaction as becoming vault), not only in Meili.
- No backfill: existing done HTML rows stay with `page_text` NULL until the person archives again.
- Prefer existing workspace deps (`scraper` et al.) for extraction; add a crate only if reader-mode quality needs it. Builder chooses the algorithm within that rule.
- Shared categories: invitees who can already open the archive may also match on the **owner’s** indexed page body via `meili_visibility_filter` (`category_ids IN […]`). **Accepted on purpose** for this slice; do not special-case `page_text` for owners only.

## Approaches considered

1. **Postgres column + Meili field (chosen).** Extract on HTML finish → store on the HTML `bookmark_archives` row → attach via an API upsert helper that loads `html_page_texts` only when indexing. Survives `add_or_replace`; supports later Markdown download without redoing storage.
2. **Meili-only patch.** Less schema; fragile under full replace and admin reindex.
3. **Re-extract on every index.** No stored text; every bookmark edit pays disk/CPU; fights the “no backfill” spirit on reindex.
4. **Load `page_text` in `ViewBatch` (rejected).** Would pull up to ~100k chars per card on every library/dashboard/search hydration and then discard it via `serde(skip)`.

## Design

### Data model

- Migration: add nullable `page_text TEXT` to `bookmark_archives`.
- Meaningful only for `format = 'html'` and non-vault bookmarks. PDF/file rows stay NULL.
- Truncate extracted text to about **100 000** **characters** (not bytes) before store (`PAGE_TEXT_MAX`).
- Do not expose `page_text` on public bookmark JSON. `BookmarkView` may carry `#[serde(skip)] page_text` filled **only** by the Meili upsert helper (empty in normal list/view builders).

### Extraction

- Pure function in `linkdean_core` (`extract_page_text(html: &str) -> String`): never panics; may return empty.
- Root selection (builder call): use `main` if present, else `body`, unless there is **exactly one** `article` — then use that article (avoids starving multi-article feeds/indexes).
- Strip `script`, `style`, `nav`, `footer`, `aside`; collect text; collapse whitespace; truncate by Unicode scalar values to `PAGE_TEXT_MAX`.
- Unit tests: body kept and chrome dropped; empty/garbage short; ASCII and multibyte (`é`) over-cap truncation.
- Runs in the API archive worker on plaintext HTML bytes when the capture target is non-vault. Vault captures skip extract entirely.

### Write path (archive finish)

1. Non-vault HTML copy succeeds → extract → pass `page_text` into `finish_archive` for the HTML row.
2. At the start of the `finish_archive` transaction: `SELECT is_vault FROM bookmarks WHERE id = $1 FOR SHARE`. Write `page_text` only when `is_vault` is false **and** format is HTML **and** `key_vault_id` is none. Otherwise store NULL.
3. Replace: new extract overwrites previous `page_text` (or clears it when empty / vault / non-HTML).
4. Extract empty or error → archive still `done`; `page_text` NULL.
5. After **every** successful non-vault `finish_archive` (HTML, PDF-only, or `file` that deletes the HTML row), run the Meili upsert helper so a replaced HTML body (or deleted HTML row) cannot leave stale Meili `page_text`.
6. Slice-1 note (out of scope to fix here): the same finish-vs-vault-move race can leave the **HTML file** on disk unencrypted. Add a `Known issue:` comment at the finish/rekey boundary pointing at a later fix (lock or rekey after finish).

### Search / index path

- Extend `BookmarkDoc` with `page_text: String` (default empty).
- `ensure_index`: append `page_text` last in searchable attributes (title/url/tags keep priority).
- API module helper (e.g. `crate::search_index::upsert` / `upsert_many`): load `html_page_texts` for the ids, set `view.page_text`, call `SearchClient::upsert_bookmark(s)`. All former direct `state.search.upsert_bookmark(s)` call sites (bookmarks, categories, trash, vault `joined_search`, import, admin reindex, search stale repair, archive finish, delete-archive) go through this helper. Guard with a test or CI grep so a missed site cannot silently wipe body text on the next edit.
- Admin reindex must use the helper so body search survives a full rebuild.
- `SearchClient::search`: retrieve only `id` (or exclude `page_text`) so hits do not ship megabytes of body per keystroke. Cards still come from Postgres.
- No API/UI change for hit shape this slice.

### Vault and lifecycle

| Event | `page_text` (HTML row) | Meili |
| --- | --- | --- |
| Non-vault HTML archive done | Set / replace / NULL if empty; gated by `is_vault FOR SHARE` | Upsert via helper (includes `page_text`) |
| Non-vault PDF-only or `file` finish | HTML row gone or unchanged; `file` finish deletes HTML → text gone | Upsert via helper (empty body if HTML row gone) |
| Vault capture | Leave NULL; do not extract | No upsert (vault) |
| Finish after bookmark became vault | Forced NULL via `is_vault` check | No upsert / skip vault |
| Move into vault (`left_search`) | **Clear in DB transaction** | `delete_bookmark` (existing) |
| Move out of vault (`joined_search`) | Stay NULL until next HTML archive | Upsert via helper without body until then |
| Delete archive (bookmark kept) | Cleared with rows | Immediate upsert via helper (empty `page_text`) |
| Purge / permanently delete bookmark | Cascade | `delete_bookmark` as today |
| Trash | Unchanged in DB | Delete / restore via helper (restore includes `page_text` if present) |

### Failures

- Extraction must not fail the archive. Log at debug/warn if useful; user sees a normal successful archive.
- Meili upsert failure after DB write: warn only; DB remains source of truth for the next upsert.

### Testing

- `core`: extract fixtures (chrome dropped, multibyte cap, single-article vs main/body).
- `db`: HTML finish stores text; encrypted / vault-row finish stores NULL; finish after SQL `is_vault=true` stores NULL; move into vault clears text; `file` finish clears HTML text; `delete_archives` leaves no text.
- API / search: helper attaches text for upsert and reindex; search retrieves only ids; vault upsert no-ops; grep/test that direct `upsert_bookmark` call sites outside the helper are gone.
- No new Playwright requirement for this slice.

### Docs when shipping

- `docs/design/archive.md`: short **Page text (search)** section; keep Markdown download under Later/Downloads; note shared-category body matches are intentional.
- `docs/product/roadmap.md`: mark page-text + Meili as shipped under item 12; leave download / media / versions.
- `docs/product/features.md` + `docs/CHANGELOG.md` under Unreleased.
- Public copy in `linkdean-infra/apps/docs/docs/roadmap.md` if the public wording changes.

## Left to the builder

- Exact extract DOM details within the root-selection rule above.
- Exact helper module name and whether `BookmarkDoc::from_view` reads `view.page_text` or takes a second argument.
- Grep vs unit-test shape for the “no direct upsert” guard.
- Migration number: next free on `main` at implement time.

## Success

- New non-vault HTML archives make page body terms findable in library search.
- Vault bookmarks never contribute body text; move-into-vault and finish-after-vault-move both leave Postgres and Meili without body text.
- Shared-folder invitees may match on the owner’s page body (same access as opening the archive).
- Existing HTML archives without re-archive do not suddenly gain body search (accepted for this slice).
- Deleting the HTML archive (or replacing it with a `file` copy) stops body matches even though the bookmark remains.
- Library lists and search responses do not load or return page bodies.
