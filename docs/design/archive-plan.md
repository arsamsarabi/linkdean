# Archive and downloads: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first slice of [archive and downloads](./archive.md). People archive a bookmark as single-file HTML and/or PDF, or keep the file itself, and then open, download, archive again or delete the copy.

**Architecture:** A `bookmark_archives` table is a durable queue and the record of each copy. A runner task in the API process claims queued bookmarks and captures them through the shared Chrome. It converts the MHTML snapshot to one HTML file in the worker crate, encrypts vault copies with the vault key, and writes files under `ARCHIVE_DIR`. Routes serve copies with a sandbox CSP. The web app adds a dialog, a viewer route, a drawer section, badges, a Quick peek fallback, a Settings tab and admin limits.

**Tech stack:** Rust (axum, sqlx, chromiumoxide 0.8, `lol_html` (new)), Postgres, React with Mantine and TanStack Router and Query, vitest.

**Conventions:**
- Work in `../linkdean-archive` on `feat/archive`.
- Commit each task when green and push it.
- `cargo test -j 2` with `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test`, sharing `CARGO_TARGET_DIR` with the main checkout.
- `cargo fmt` runs in `rust:1.94` at the end.
- Web checks: prettier, `tsc -b` and oxlint on the changed files.

---

## File map

| File | Responsibility |
| --- | --- |
| `migrations/059_bookmark_archives.sql` | Table, platform limits, user settings |
| `crates/core/src/models/archive.rs` (new, re-exported) | `ArchiveFormat`, `ArchiveStatus`, `BookmarkArchiveView`, `ArchiveSummary`, `RequestArchive`, `BulkArchive` |
| `crates/core/src/crypto/mod.rs` | `encrypt_archive`, `decrypt_archive` (AAD `LDa1`) |
| `crates/db/src/repo/archives.rs` (new) | Request, list, claim, finish, fail, requeue, delete, usage, view summaries |
| `crates/worker/src/archive/mhtml.rs` (new) | MHTML parser and single-file HTML converter (pure, unit-tested) |
| `crates/worker/src/archive/mod.rs` (new) | `probe`, `fetch_file`, `capture_page` (Chrome: scroll, MHTML, PDF) |
| `crates/api/src/archive_runner.rs` (new) | Queue loop, encryption, file swap, vault re-keying, purge helper |
| `crates/api/src/routes/archives.rs` (new) | `POST/GET/DELETE /bookmarks/{id}/archive`, `GET …/{format}`, `POST /bookmarks/archive` |
| `crates/api/src/{config,state,security,quotas,main}.rs`, `routes/{admin,bookmarks,trash,vault,settings}` | Wiring |
| `apps/web/src/schemas/archive/archive.ts` and `src/api/api.ts` | Schemas and client |
| `apps/web/src/hooks/archive/*` | `useArchives`, `useRequestArchive`, `useDeleteArchive`, `useBulkArchive` |
| `apps/web/src/components/archive/*` | `ArchiveDialog`, `ArchiveSection`, `ArchiveSettingsPanel`, `archiveUrls` |
| `apps/web/src/routes/_app/archive.$id.tsx` and `src/pages/archive/ArchivePage.tsx` | Viewer |
| Menu, card badges, drawer, peek, bulk bar, settings, admin panels | Entry points |
| `deploy/docker/*.yml`, `docs/self-hosting/environment.md`, features, changelog | Deploy and docs |

---

### Task 1: Migration

**Files:** Create `migrations/059_bookmark_archives.sql`. First check that 059 is still free on `origin/main`.

- [ ] Write the migration:

```sql
-- Archived copies of bookmarks (roadmap item 12). One row per bookmark and format; the row is
-- both the queue entry and the record of the stored copy.
CREATE TABLE bookmark_archives (
    bookmark_id  UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    format       TEXT NOT NULL CHECK (format IN ('html', 'pdf', 'file')),
    status       TEXT NOT NULL CHECK (status IN ('queued', 'running', 'done', 'failed')),
    requested_by UUID REFERENCES users(id) ON DELETE SET NULL,
    -- File under ARCHIVE_DIR/{bookmark_id}/ holding the current copy; NULL until one exists.
    file_name    TEXT,
    bytes        BIGINT NOT NULL DEFAULT 0,
    content_type TEXT,
    -- NULL for vault bookmarks, so the address stays encrypted.
    final_url    TEXT,
    -- The vault whose key encrypted the file; NULL = stored in plain.
    key_vault_id UUID,
    nonce        BYTEA,
    captured_at  TIMESTAMPTZ,
    error        TEXT,
    queued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at   TIMESTAMPTZ,
    PRIMARY KEY (bookmark_id, format)
);
CREATE INDEX bookmark_archives_queued ON bookmark_archives (queued_at) WHERE status = 'queued';

ALTER TABLE platform_settings
    ADD COLUMN archives_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    -- Per-account archive storage; NULL = unlimited. The hosted beta sets 500 MB.
    ADD COLUMN max_archive_bytes BIGINT CHECK (max_archive_bytes IS NULL OR max_archive_bytes >= 0),
    ADD COLUMN max_archive_item_bytes BIGINT NOT NULL DEFAULT 26214400
        CHECK (max_archive_item_bytes BETWEEN 1048576 AND 1073741824);

ALTER TABLE user_settings
    ADD COLUMN archive_new BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN archive_formats TEXT[] NOT NULL DEFAULT ARRAY['html']
        CHECK (archive_formats <@ ARRAY['html', 'pdf'] AND cardinality(archive_formats) > 0);
```

- [ ] Run `cargo test -j 2 -p linkdean-db --test vaults`. Expected: migrations apply and the tests pass.
- [ ] Commit: "Add the bookmark archives table and archive settings".

### Task 2: Core models and archive crypto

**Files:** Create `crates/core/src/models/archive.rs` and add `mod archive; pub use archive::*;` in `models/mod.rs`. Modify `models/mod.rs`: `UserSettings` and `UpdateSettings` (`archive_new`, `archive_formats`); `PlatformSettings` and `UpdatePlatformSettings` (`archives_enabled`, `max_archive_bytes`, `max_archive_item_bytes`); `ResolvedLimits.max_archive_bytes`; `QuotaUsageView.archive_bytes`; `BookmarkView.archive: Option<ArchiveSummary>` (`serde(default, skip_serializing_if)`). Modify `crypto/mod.rs`.

- [ ] Models:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat { Html, Pdf, File }
// as_str / parse like LinkCheckStatus; `ext()` -> "html" | "pdf" | "bin"; `tag()` -> b'h' | b'p' | b'f'

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveStatus { Queued, Running, Done, Failed }

/// One stored or pending copy, as the drawer shows it.
pub struct BookmarkArchiveView {
    pub format: ArchiveFormat, pub status: ArchiveStatus, pub bytes: i64,
    pub content_type: Option<String>, pub captured_at: Option<DateTime<Utc>>,
    /// Set while a newer capture failed but an older copy is kept, and on failure.
    pub error: Option<String>,
    /// Whether a copy can be opened now (done earlier, even if a re-archive failed).
    pub available: bool,
}

/// Compact form on `BookmarkView`, for badges and menus.
pub struct ArchiveSummary {
    pub formats: Vec<ArchiveFormat>,              // with a stored copy
    pub captured_at: Option<DateTime<Utc>>,       // newest stored copy
    pub pending: bool,                            // any queued or running
    pub failed: bool,                             // latest attempt of any format failed
}

pub struct RequestArchive { pub formats: Vec<ArchiveFormat> }       // html and/or pdf
pub struct BulkArchive { pub ids: Vec<Uuid>, pub formats: Vec<ArchiveFormat> }
```

- [ ] Crypto, beside `encrypt_payload`:

```rust
fn archive_aad(vault_id: Uuid, bookmark_id: Uuid, format: ArchiveFormat) -> [u8; 37] {
    let mut aad = [0u8; 37];
    aad[..4].copy_from_slice(b"LDa1");
    aad[4..20].copy_from_slice(vault_id.as_bytes());
    aad[20..36].copy_from_slice(bookmark_id.as_bytes());
    aad[36] = format.tag();
    aad
}
pub fn encrypt_archive(key: &[u8; 32], vault_id: Uuid, bookmark_id: Uuid, format: ArchiveFormat, bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>)>
pub fn decrypt_archive(key: &[u8; 32], vault_id: Uuid, bookmark_id: Uuid, format: ArchiveFormat, ct: &[u8], nonce: &[u8]) -> Result<Vec<u8>>
```

- [ ] Add tests: a round trip; and opening fails for a different bookmark, vault or format.
- [ ] Fix every `BookmarkView {` literal (two in `bookmarks.rs`) with `archive: batch.archives.get(&b.id).cloned()`. The `ViewBatch` field lands in Task 3; until then use `None`.
- [ ] Run `cargo test -j 2 -p linkdean-core`, then commit "Add archive models and vault archive encryption".

### Task 3: DB repo

**Files:** Create `crates/db/src/repo/archives.rs` and export it from `repo/mod.rs`. Modify `bookmarks.rs` (`ViewBatch.archives`), `platform.rs` (columns and setters), `settings` repo (new fields), `quotas.rs` (`archive_bytes_for_owner`, `METER_ARCHIVE`). Test: `crates/db/tests/archives.rs`.

Functions:

```rust
/// Queue `formats` for a bookmark the user can write. Refuses vault bookmarks whose vault the
/// caller has not unlocked, an archive while archiving is off, and an archive with no room left.
pub async fn request_archive(pool, user_id, bookmark_id, formats: &[ArchiveFormat], keyring: &VaultKeyring) -> Result<Vec<BookmarkArchiveView>>
// - bookmark_can_write, else NotFound (or Forbidden when readable): same as update_bookmark
// - platform archives_enabled else BadRequest("Archiving is turned off on this server.")
// - formats non-empty, only html/pdf
// - quota: owner's done bytes >= max_archive_bytes -> QuotaExceeded { meter: METER_ARCHIVE, .. }
// - INSERT .. ON CONFLICT (bookmark_id, format) DO UPDATE SET status='queued', error=NULL,
//   queued_at=now(), requested_by=$user  (keeps file_name/bytes: the old copy stays until replaced)

pub async fn list_archives(pool, viewer_id, bookmark_id) -> Result<Vec<BookmarkArchiveView>>   // read access
pub async fn archive_row(pool, bookmark_id, format) -> Result<Option<ArchiveRow>>              // serving
pub struct ArchiveRow { bookmark_id, format, status, file_name, bytes, content_type, final_url, key_vault_id, nonce, captured_at, error }

/// Claim the oldest queued bookmark: every queued row of it goes to running together, so one
/// Chrome visit makes all its formats.
pub async fn claim_next_archive(pool) -> Result<Option<(Uuid, Vec<ArchiveFormat>)>>
// UPDATE bookmark_archives SET status='running', started_at=now()
// WHERE status='queued' AND bookmark_id = (SELECT bookmark_id FROM bookmark_archives
//   WHERE status='queued' ORDER BY queued_at LIMIT 1 FOR UPDATE SKIP LOCKED)
// RETURNING bookmark_id, format

pub struct StoredCopy { pub format, pub file_name: String, pub bytes: i64, pub content_type: String, pub final_url: Option<String>, pub key_vault_id: Option<Uuid>, pub nonce: Option<Vec<u8>> }
/// Record new copies; returns the file names they replace, for the caller to delete. A `file`
/// copy replaces the bookmark's html and pdf rows, and the other way round.
pub async fn finish_archive(pool, bookmark_id, copies: &[StoredCopy]) -> Result<Vec<String>>
/// Mark running rows failed; rows with an older copy keep it (status back to done, error set).
pub async fn fail_archive(pool, bookmark_id, formats: &[ArchiveFormat], error: &str) -> Result<()>
/// After a restart nothing is running: put those rows back in the queue.
pub async fn requeue_running_archives(pool) -> Result<u64>
/// Delete every copy (write access); returns file names to remove.
pub async fn delete_archives(pool, user_id, bookmark_id) -> Result<Vec<String>>
pub async fn archive_bytes_for_owner(pool, user_id) -> Result<i64>   // SUM(bytes) WHERE status='done' or file_name IS NOT NULL, over owned bookmarks
pub async fn archive_owner_and_limit(pool, bookmark_id) -> Result<(Uuid, Option<i64>, i64)> // owner, quota, item cap
/// Rows whose file is encrypted under another vault than the bookmark is in now.
pub async fn archives_needing_rekey(pool, bookmark_ids: &[Uuid]) -> Result<Vec<(ArchiveRow, Option<Uuid>)>>
pub async fn set_archive_key(pool, bookmark_id, format, file_name, key_vault_id: Option<Uuid>, nonce: Option<Vec<u8>>, clear_final_url: bool) -> Result<Option<String>>
```

Rules worth a test each (`crates/db/tests/archives.rs`, using helpers like those in `vault_moves.rs`):

- [ ] An owner request queues html and pdf rows. A read invitee gets Forbidden; a write invitee queues, and the bytes count for the owner.
- [ ] Archiving off: BadRequest. Over quota: QuotaExceeded.
- [ ] `claim_next_archive` claims one bookmark's rows; a second call skips them.
- [ ] `finish_archive` replacing an earlier copy returns the old file name. A `file` copy removes the html and pdf rows.
- [ ] `fail_archive` on a row with an old copy keeps status done, with the error set.
- [ ] `requeue_running_archives` puts running rows back.
- [ ] `ViewBatch` summary: formats with copies, pending and failed flags.
- [ ] A purged bookmark cascades its rows.
- [ ] Commit: "Add the archive queue and records".

### Task 4: MHTML to single-file HTML (worker, pure)

**Files:** Add `lol_html = "2"` to workspace and worker deps; check the current version on crates.io first and pin the exact minor. Create `crates/worker/src/archive/mhtml.rs`. Test: in-module `#[cfg(test)]` with fixture strings.

API:

```rust
pub struct Limits { pub max_resource: usize, pub max_total: usize }
#[derive(Debug, PartialEq)] pub enum ConvertError { NotMhtml, NoDocument, TooBig(usize) }
pub fn to_single_html(mhtml: &[u8], limits: &Limits) -> Result<Vec<u8>, ConvertError>
```

Steps inside:

1. `parse(mhtml)`:
   - Unfold the header lines.
   - Read `boundary` from `Content-Type`.
   - Split the body on `--{boundary}`.
   - For each part, parse its headers (`Content-Type`, `Content-Location`, `Content-ID`, `Content-Transfer-Encoding`).
   - Decode `base64` (whitespace stripped) and `quoted-printable` (`=\r\n` soft breaks, `=XX`).
   - The first `text/html` part is the document; its `Content-Location` is the base URL.
2. Resource map: `HashMap<String, (content_type, bytes)>` keyed by absolute location, and by `cid:{id}` with the angle brackets removed.
3. `inline_css(css, base, map, depth)`:
   - Rewrite each `url(…)` (quoted or bare) to a `data:` URI when the resolved URL is in the map and within `max_resource`.
   - Inline `@import` CSS recursively, to depth 3.
4. `lol_html` rewrite of the document:
   - remove `script, noscript, iframe, frame, frameset, object, embed, applet, base, meta[http-equiv], link[rel=preload], link[rel=modulepreload], link[rel=prefetch]`
   - every element: drop `on*` attributes and `javascript:` URLs in `href`, `src` and `action`
   - `img[src]`, `source[src]`, `video[poster]`, `input[type=image][src]`: replace with a data URI
   - `img[srcset]`, `source[srcset]`: keep only candidates found in the map, as data URIs; drop the attribute if none are left
   - `link[rel~=stylesheet][href]`: replace with a `data:text/css;base64` of the inlined CSS
   - `link[rel~=icon][href]`: replace with a data URI
   - `[style]`: run through `inline_css`
   - `style` text: buffer the chunks, then replace the last one with `inline_css(buffer)`
   - `a[href]` and `area[href]`: make absolute against the base
   - `head`: prepend `<meta charset="utf-8"><base target="_blank">`
5. If the output is longer than `max_total`, return `TooBig`.

Tests (fixtures built in the test with a small `mhtml(parts)` helper that writes CRLF parts):

- [ ] An image part becomes `src="data:image/png;base64,…"`.
- [ ] CSS `url(bg.png)`, relative to the stylesheet's own URL, is inlined, and the stylesheet link becomes a data URI.
- [ ] `<script>`, an `onclick` attribute and an `<iframe>` are gone; `javascript:` hrefs are removed.
- [ ] A part over `max_resource` keeps its original URL, which the CSP then blocks.
- [ ] Quoted-printable soft breaks and `=3D` decode.
- [ ] Relative `<a href>` becomes absolute.
- [ ] A document over `max_total` gives `TooBig`. Input that is not MHTML gives `NotMhtml`.
- [ ] Commit: "Convert Chrome MHTML snapshots to single-file HTML".

### Task 5: Capture (worker, Chrome and HTTP)

**Files:** Create `crates/worker/src/archive/mod.rs` and add `pub mod archive;` in `lib.rs`. Make `resolve_browser_ws`, `guard_page_requests`, `is_challenge_page` and `read_capped` `pub(crate)`.

```rust
pub enum Probe { Page, File { content_type: String }, Gone(u16) }
/// GET through the guarded client, reading headers only. 404 and 410 mean gone; a success with a
/// type other than HTML or XHTML is a file; anything else (403, 429, errors) is left to Chrome.
pub async fn probe(url: &str) -> Probe
/// Download a file up to `max` bytes; the type is checked from the bytes for images and PDFs.
pub async fn fetch_file(url: &str, max: usize) -> anyhow::Result<(Vec<u8>, String)>
pub struct Captured { pub html: Option<Vec<u8>>, pub pdf: Option<Vec<u8>>, pub final_url: String }
/// One tab: guard, navigate (20 s), settle, scroll up to ten screens (300 ms each) and back to
/// the top, refuse challenge pages, then `Page.captureSnapshot` (MHTML) -> `to_single_html`
/// and/or `Page.printToPDF` (print_background), each bounded by 30 s.
pub async fn capture_page(browser_http: &str, url: &str, want_html: bool, want_pdf: bool, limits: &mhtml::Limits) -> anyhow::Result<Captured>
/// The type a stored file is served with inline, or None for attachment-only.
pub fn inline_type(content_type: &str, bytes: &[u8]) -> Option<&'static str>  // image sniff, %PDF-, audio/*, video/*
```

- [ ] Unit tests for `inline_type`: SVG gives None; PNG bytes give `image/png`; `%PDF-` gives `application/pdf`; `text/html` gives None.
- [ ] Add an ignored test `captures_a_real_page` (`#[ignore = "needs Chrome"]`, reads `CHROME_URL`): capture `https://example.com/` and expect HTML containing "Example Domain", no `<script`, and a PDF starting with `%PDF-`.
- [ ] Run the ignored test once against the dev Chrome on :9222.
- [ ] Commit: "Capture pages and files for archives".

### Task 6: Runner, config and state (API)

**Files:**
- Modify `config/mod.rs`: `archive_dir` (`ARCHIVE_DIR`, default `./data/archives`) and `archive_concurrency` (`ARCHIVE_CONCURRENCY`, default 2, 1 to 8).
- Modify `state/mod.rs`: create the dir, add `archive_wake: Arc<Notify>`, `archive_dir()`.
- Modify `vault_keys.rs`: `user_key(user_id, vault_id) -> Option<[u8; 32]>` from any unexpired session, without marking it used.
- Create `archive_runner.rs`. Modify `main.rs`: requeue, then spawn the runner when `browser_ws_url` is set.

Runner:

```rust
pub async fn run(state: AppState)   // semaphore(archive_concurrency); loop { claim_next_archive; if None { notified or 30 s }; spawn job }
async fn job(state, bookmark_id, formats)
// 1. raw bookmark row (deleted -> fail "The bookmark was deleted"). URL: plain url, or for a
//    vault bookmark decrypt the payload with vault_keys.user_key(owner, vault) (none -> fail
//    "Unlock the vault, then archive again").
// 2. limits: item cap (platform) and quota room = max_archive_bytes - used.
// 3. probe: Gone(code) -> fail "The page is gone (HTTP {code}); the earlier copy is kept" (or
//    "...gone (HTTP {code})" when there is none). File -> fetch_file -> one `file` copy.
//    Page -> capture_page(want_html, want_pdf).
// 4. each copy: over item cap -> fail "This page is too big to archive (over N MB)"; over
//    quota room -> fail "Archive storage is full". Vault: encrypt_archive with the same key.
// 5. write `{dir}/{bookmark_id}/{format}-{8 random hex}.{ext}.tmp`, fsync, rename without .tmp,
//    finish_archive, then delete the replaced files. Any error -> fail_archive + delete temp.
pub async fn rekey_after_vault_move(state, keyring: &VaultKeyring, bookmark_ids: &[Uuid])
// for each archives_needing_rekey row: read file, decrypt with key_vault_id key (if any), encrypt
// with the bookmark's vault key (if any), write a new file, set_archive_key(clear_final_url when
// entering a vault), delete the old file. A missing key logs a warning and leaves the row.
pub async fn purge_bookmark_archives(dir: &Path, bookmark_id: Uuid)  // remove_dir_all, ignore NotFound
```

- [ ] Unit test for the temp-and-rename helper in a temp dir: a failed write leaves no file behind.
- [ ] Commit: "Run archive captures from a durable queue".

### Task 7: Routes, serving and wiring

**Files:**
- Create `routes/archives.rs` and merge it in `routes/bookmarks/mod.rs` or `routes/mod.rs`.
- Modify `security.rs`: keep an existing `X-Frame-Options` like the CSP; the app CSP gets `frame-src 'self' https://challenges.cloudflare.com https:`.
- Modify `quotas.rs` (`archive_bytes`, limit).
- Modify `routes/admin/mod.rs` (the three platform fields).
- Modify the settings route (the two user fields; formats must be non-empty and only html or pdf).
- Modify the bookmark create route: when the creator has `archive_new` and archiving is on, call `request_archive` with their formats. Errors are logged, not returned; over quota records a failed row "Archive storage is full".
- Modify the trash purge, maintenance account purge and permanent delete paths: call `purge_bookmark_archives` wherever `purge_preview_assets` runs.
- Modify the vault move route: after `Moved`, call `rekey_after_vault_move` with the moved ids.
- Regenerate `docs/api/openapi.json` (`UPDATE_OPENAPI=1 cargo test -p linkdean-api committed_spec_is_current`).

Routes:
- `POST /bookmarks/{id}/archive` body `RequestArchive` → `Vec<BookmarkArchiveView>`, then `archive_wake.notify_one()`.
- `POST /bookmarks/archive` body `BulkArchive` (max 500 ids) → `{ "ok": n, "failed": [{ "id", "error" }] }`.
- `GET /bookmarks/{id}/archive` → `Vec<BookmarkArchiveView>`.
- `GET /bookmarks/{id}/archive/{format}` serves the copy:
  - Access: `get_bookmark_raw`, falling back to the dashboard-share path in `get_bookmark_for_assets` for non-vault rows. For an encrypted copy, the keyring must hold `key_vault_id`, else VaultLocked.
  - Headers: `Cache-Control: private, no-store` and `X-Content-Type-Options: nosniff`.
  - html: `Content-Type: text/html; charset=utf-8`, the sandbox CSP from the design with `sandbox allow-popups allow-popups-to-escape-sandbox`, and `X-Frame-Options: SAMEORIGIN`.
  - pdf, and files whose `inline_type` is some: that type, inline, with no sandbox CSP (Chrome refuses PDFs under a sandbox) but `default-src 'none'; frame-ancestors 'self'`.
  - Anything else: `application/octet-stream`.
  - `?download=1`, or no inline type: `Content-Disposition: attachment; filename="<host>-<YYYY-MM-DD>.<ext>"`. The host comes from the bookmark view's URL; the date from `captured_at`.
- `DELETE /bookmarks/{id}/archive` → `{ ok: true }`, removing the files.

Tests (`crates/api` unit tests where the code is pure, DB tests otherwise):

- [ ] Header builder: html gets sandbox, SAMEORIGIN and nosniff; svg and unknown types are attachment-only; the download name is right.
- [ ] Security middleware keeps a route's own `X-Frame-Options`.
- [ ] Commit: "Serve, request and delete archives over the API".

### Task 8: Web data layer

**Files:**
- Create `src/schemas/archive/archive.ts` and export it from `schemas/index.ts`.
- `bookmarkSchema.archive` (nullish).
- `api.ts`: `archives(id)`, `requestArchive(id, formats)`, `bulkArchive(ids, formats)`, `deleteArchive(id)`.
- `src/lib/archive/archiveUrls.ts`: `archiveFileUrl(id, format, download?)`.
- Hooks in `src/hooks/archive/`:
  - `useArchives(id)`, which polls every 3 s while any row is pending
  - `useRequestArchive`, `useBulkArchive` and `useDeleteArchive`, which invalidate `['archives', id]`, `['bookmarks']` and `['quota']`
- Settings and platform schemas gain the new fields.

- [ ] Test `useArchives`: polls while pending, stops when done.
- [ ] Commit.

### Task 9: Web UI

**Files and behavior:**

- `components/archive/ArchiveDialog.tsx`:
  - HTML checked, PDF unchecked.
  - If the bookmark has a copy: a yellow note, "This replaces the copy from {date}. If archiving fails, the current copy is kept."
  - Submit calls `requestArchive` and shows a notification: "Archiving… you can keep working".
  - Used for one bookmark and for bulk (`ids`).
- `components/archive/ArchiveSection.tsx` in the bookmark drawer (edit mode):
  - status line: Queued / Archiving… / the failure reason in red
  - capture date and size
  - buttons: Open, Download (one per format), Archive again, Delete (confirm)
- `BookmarkCardMenu`:
  - **Archive…** when `canWrite`
  - **Open archive** when `archive.formats` has html, pdf or file
- `BookmarkCardBadges`:
  - an archive icon (Tooltip "Archived {date}") when formats is non-empty
  - when `link_check_status` is broken or unreachable and a copy exists, the broken badge links to the archive viewer, with the label "Page gone: open archived copy"
- `pages/archive/ArchivePage.tsx` and `routes/_app/archive.$id.tsx` (then run `npx tsr generate`):
  - a header with the title, "Archived {date}", the live link, and download buttons
  - body: an iframe of the html copy (`sandbox="allow-popups allow-popups-to-escape-sandbox"`), else PDF and file links opening `archiveFileUrl` in a new tab
- `PeekPanel`:
  - `PeekTarget.archived?: boolean`, set by callers from `archive.formats.includes('html')`
  - when `frameBlocked && archived`, render the archive iframe with a note "Showing the archived copy from {date}"
- `LibraryBulkBar`: an **Archive** button opening the dialog with `ids`. Like Move to vault, it is disabled when nothing writable is selected.
- Settings: a new tab `archive` (`SETTINGS_TABS`, icon `IconArchive`) with `ArchiveSettingsPanel`: the switch "Archive new bookmarks automatically", default-format checkboxes (at least one), and usage "{used} of {quota}" (or "{used} used").
- Admin platform settings panel: "Archiving" switch, "Archive storage per account (MB)" (empty = unlimited), "Largest archived copy (MB)".

- [ ] Tests:
  - the dialog shows the overwrite note only when a copy exists, and sends the formats
  - the menu items show by access and copy state
  - the badge links to the viewer when broken and archived
  - peek shows the archive when blocked and archived
  - the settings panel saves the formats
- [ ] Run `npx tsr generate`, prettier, `tsc -b`, oxlint and vitest. Commit per component group.

### Task 10: Deploy and docs

- [ ] `deploy/docker/docker-compose.yml` and `.dev.yml`: add an `archives` volume at `/app/data/archives` and `ARCHIVE_DIR: /app/data/archives` on `api`.
- [ ] `docs/self-hosting/environment.md`: `ARCHIVE_DIR` and `ARCHIVE_CONCURRENCY`.
- [ ] `docs/self-hosting/operations.md`: back up the archives volume.
- [ ] `docs/product/features.md`: a new section, Archive.
- [ ] `docs/CHANGELOG.md`: Added.
- [ ] `docs/product/roadmap.md`: mark the first slice as shipped, keeping the later bullets.
- [ ] `docs/design/archive.md`: link this plan.
- [ ] Run the full Rust suite with the DB, fmt in rust:1.94, the full web suite, then push and open the PR.
- [ ] Separate, in linkdean-infra: the restic paths and the archives volume on the homelab, and the beta's `max_archive_bytes` of 500 MB set in Admin after deploy. Listed for the owner; not done in this repo.

## Self-review

- Design coverage:
  - trigger (Tasks 7 and 9), formats (2, 3, 9), capture (4, 5), versions and warning (3, 9), media and item cap (4, 5, 6), quota (3, 6, 7)
  - vaults (2, 6, 7), sharing (3, 7), retention (7), serving (7), the API (7), the interface (9), deploy (10), testing (every task)
- Shortcuts, to mark as `shortcut:` comments where they land:
  - Search results from Meilisearch carry no archive summary, so they show no badge.
  - Imports do not archive automatically.
  - One API process is assumed: the requeue at boot treats every running row as orphaned.
