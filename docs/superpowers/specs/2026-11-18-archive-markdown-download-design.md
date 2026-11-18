# Archive Markdown download — design

Date: 2026-11-18  
Roadmap: item **12** (Downloads: reader-mode Markdown)  
Status: approved in conversation (approach B)

## Intent

Someone with an HTML archive can download a readable Markdown version of that page (headings and paragraphs) for offline reading or pasting elsewhere. Export-with-archives zip stays a later slice.

## Goals

- From a **done HTML** copy, convert on the fly and offer a `.md` download.
- Same access as today’s copy download (anyone who can open/download the archive).
- Vault: decrypt in memory while unlocked, convert, respond; never store plaintext Markdown.
- Filename matches existing pattern: `<site>-<date>.md`.

## Non-goals

- Full export zip including archives
- Links, image refs, or richer fidelity beyond headings and paragraphs
- PDF or `file` → Markdown
- Persisting Markdown on disk or in Postgres
- Changing search, `page_text`, or Meilisearch
- Requiring re-archive or backfill for older HTML copies (on-demand conversion covers them)

## Anti-goals

- Writing vault page bodies to disk or DB as Markdown
- Failing or replacing the HTML archive if conversion is empty or poor
- Adding a new archive format row (`md`) alongside `html` / `pdf` / `file`

## Constraints

- Prefer workspace `scraper` (already used by `page_text`); do not add a Markdown crate unless conversion quality clearly fails fixtures.
- Reuse the existing copy load path (auth, vault unlock, file read, decrypt).
- Cap converted output length (reuse or mirror `PAGE_TEXT_MAX` = 100_000 Unicode scalars) so a huge page cannot blow memory on download.
- No change to archive quotas or storage accounting (nothing new is stored).

## Approaches considered

1. **Hand out stored `page_text` as `.md`.** Fast; no vault; fails for older HTML without `page_text`; flat text, not headings.
2. **Convert the HTML copy on download (chosen).** Matches “taken from the copy”; works for vault while unlocked and for any done HTML file; slight CPU per download; no new column.
3. **Persist Markdown at archive finish.** Instant later downloads; duplicates storage; vault plaintext-on-disk rules; old copies still need on-demand fallback.

## How it works

### API

Extend `GET /bookmarks/{id}/archive/{format}` (same handler as today’s copy).

- Query: keep `download`; add `as=md` (reject other `as` values).
- `as=md` is only valid when `{format}` is `html`, the row is available (`done` with a file), and the caller may read the copy.
- Flow: existing load + decrypt → convert plaintext HTML bytes → response body is Markdown UTF-8.
- Headers: `Content-Type: text/markdown; charset=utf-8`; `as=md` always uses `Content-Disposition: attachment` with `<site>-<date>.md` (never inline), whether or not `download` is present. UI still passes `download=1` for consistency with other downloads. Use the same host/date naming helpers as HTML downloads.
- Without a usable HTML body after conversion (empty string): return a clear client error (prefer the same family as nearby archive errors; builder chooses `400` vs `404`) with a short message such as “No readable text in this copy.” Do not 500.

### Conversion (`linkdean-core`)

New pure function next to page-text extract (e.g. `html_to_reader_markdown`):

- Same article root selection as search extract: single `article` → else `main` → else `body`.
- Emit ATMarkdown headings for `h1`–`h6`, paragraphs as blank-line-separated blocks.
- Skip chrome: `nav`, `footer`, `aside`, `script`, `style` (same spirit as `page_text`).
- Never panics; may return empty.
- Cap at `PAGE_TEXT_MAX` (or a shared constant) by Unicode scalars.

Left to the builder: exact newline rules for nested blocks, whether list items become plain paragraphs this slice, and the empty-result HTTP status — as long as goals and anti-goals hold.

### Vault

Unchanged decrypt in `get_copy`. Conversion runs only on in-memory plaintext. Locked vault → existing `VaultLocked`. No Postgres/`page_text` involvement for vault (or for this feature at all).

### UI

- On available **HTML** archive rows (drawer `ArchiveSection`, and the archive viewer if it already exposes Download), add a **Markdown** download control next to Download.
- Wire through `archiveFileUrl` (or a sibling helper) with `download=1` and `as=md`.
- Do not show Markdown for PDF or `file` rows.

### Docs / roadmap

When shipping: update `docs/design/archive.md` (Later → note Markdown download shipped), `docs/product/features.md`, `docs/product/roadmap.md` (trim the Markdown bullet), and `docs/CHANGELOG.md` Unreleased. Public docs/roadmap only if the product list changes in a user-visible way.

## Testing

- Core: headings and paragraphs kept; chrome dropped; output capped; empty/garbage → empty or short.
- API: Markdown content-type and attachment name; vault locked refuses; `as=md` on non-html rejected; empty conversion → client error.
- Web: Markdown link present for available HTML; absent for PDF/`file`.

## Out of scope reminders

Export zip with archives, embedded media, version history, hosted tiers, import auto-archive, and hiding archive actions when archiving is admin-disabled remain later item-12 work.
