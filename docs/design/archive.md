# Archive and downloads

Requirements agreed with the product owner on 2026-11-17, in an interview. This covers the first slice of roadmap item 12. Later slices are listed at the end. The build steps are in the [implementation plan](./archive-plan.md).

## Why

Link rot is the main reason people keep bookmarks on their own server. Linkwarden, Karakeep and Raindrop.io (Pro) all keep a copy of each page; Linkdean stores previews only. The owner ranked the goals:

1. Link-rot insurance: a bookmark still works after its page dies or changes.
2. Search page contents (shipped for non-vault HTML copies; see Page text below).
3. Own my data: download copies, and later include them in the export.
4. Read offline or later, inside Linkdean.

## First slice

Snapshot, view and download.

- Archive a bookmark by hand (one or many), or automatically on save once the person turns that on.
- Keep a single-file HTML copy, a PDF, or both. HTML is the default.
- A bookmark whose URL is itself a file (PDF, image, audio, video) is kept as that file.
- Open the copy in Linkdean, download it, archive it again, delete it.
- When a site refuses to show in Quick peek, or the link checker finds the page gone, offer the copy.
- Vault bookmarks get copies encrypted with the vault key.

Not in this slice: the export zip with archives, embedded video and audio, and version history (see Later). Markdown download ships separately (see below).

## Page text (search)

After a successful **non-vault HTML** archive (new or replace), Linkdean extracts reader-mode page text from the snapshot, stores it on the HTML archive row, and indexes it in Meilisearch so library search matches words in the page body, not only title, URL, description, notes and tags.

- Vault bookmarks are never extracted, never stored as page text, and never upserted to Meili. Moving a bookmark into a vault clears stored page text and removes the Meili document.
- Deleting the archive (or replacing HTML with a `file` copy) clears body text from search even though the bookmark remains.
- No backfill: existing done HTML rows stay without page text until the person archives again.
- Shared-category invitees who can already open the archive may also match on the **owner’s** indexed page body. That is intentional for this slice; do not special-case body matches to owners only.
- Reader-mode Markdown download is separate (see Markdown download below); this section is search indexing only.

## Markdown download

Anyone who can download an HTML archive can also download a reader-mode Markdown version (`as=md` on the HTML copy route). Conversion runs on the fly from the stored HTML (headings and paragraphs); nothing new is stored. Vault copies decrypt in memory while unlocked, same as opening the HTML. Empty conversion returns a clear client error; the archive row is unchanged. PDF and original-file copies do not offer Markdown.

## Decisions

| Topic | Decision |
| --- | --- |
| Trigger | A manual **Archive…** action on a bookmark and in the bulk bar. Plus a per-account switch, **Archive new bookmarks automatically**, off by default. |
| Formats | The person picks per archive; HTML is preselected and PDF is optional. Automatic archiving uses the account's default formats, HTML unless changed. Format choice does not apply to direct files. |
| Capture method | Chrome's own MHTML snapshot, converted to one HTML file in Rust. PDF from Chrome's print. No new service. |
| Versions | Latest copy only. Archiving again warns that it replaces the copy from a given date. |
| Media | Images, styles and fonts the page loaded are inlined. A single resource over 5 MB is left out. Embedded players keep their poster image and a link. Downloading embedded video or audio is explored in a later slice. |
| Item cap | 25 MB per copy. A larger page or file fails with a clear message. |
| Quota | Per-account archive storage. 500 MB on the hosted beta; unlimited by default when self-hosting. |
| Vaults | Allowed while the vault is unlocked. Stored encrypted with the vault key and never indexed. |
| Sharing | Anyone who can read a bookmark can open and download its copy. The owner and write invitees can archive, archive again and delete; every copy counts against the owner's quota. |
| Retention | A copy stays while its bookmark is in Trash. It is deleted when the bookmark is purged or permanently deleted, when the account is deleted, or by **Delete archive**. |

## How it works

### Queue

A table `bookmark_archives` holds one row per bookmark and format (`html`, `pdf`, or `file`). Each row has:

- a status: `queued`, `running`, `done` or `failed`
- the stored size in bytes, the content type and the capture time
- the final URL after redirects
- the reason for a failure
- whether the copy is encrypted, and the nonce if it is

Requests to archive insert or reset rows to `queued`. A worker loop in the API process claims queued rows with `FOR UPDATE SKIP LOCKED` and runs at most `ARCHIVE_CONCURRENCY` captures at once (default 2), because captures share Chrome with previews. After a restart, `running` rows older than a timeout go back to `queued`, so nothing is lost.

### Capture

1. The worker first checks what the URL serves, through the existing guarded HTTP client. If it is not HTML, it downloads the file (up to the item cap) and stores it as format `file`.
2. For a page, Chrome opens it with the existing private-address request guard, waits as previews do, and scrolls down at most ten screens so lazy images load. It refuses Cloudflare challenge pages, as previews do.
3. HTML: CDP `Page.captureSnapshot` returns MHTML of everything Chrome loaded. A converter turns it into one HTML file:
   - every part is inlined as a `data:` URI, including `url()` references in CSS
   - `<script>`, `<noscript>`, inline event handlers, `<iframe>`, `<object>`, `<embed>` and `<base>` are removed
   - links point at their absolute original URLs
   - a resource over 5 MB is dropped
4. PDF: CDP `Page.printToPDF` with backgrounds.
5. A copy over the item cap fails with "This page is too big to archive (over 25 MB)."

The copy is written to a temporary file and moved into place only when it succeeded. A failed capture, a challenge page or an error status never replaces an existing good copy; the row keeps its old copy and records the failure.

### Storage

Copies live under `ARCHIVE_DIR` (a new `archives` volume), at `{bookmark_id}/{format}-{random id}`. A new copy is written under a fresh name and swapped in, so a reader never sees half a file. A vault bookmark's copy is encrypted with AES-256-GCM under the vault's data key. The associated data binds it to the vault, the bookmark and the format, as vault payloads do. Moving a bookmark into a vault encrypts its copies; moving it out decrypts them; moving it between vaults re-encrypts them, right after the move, using the key of the vault each copy is in now. An hourly sweep deletes files that no row lists any more and are over an hour old: the files of purged bookmarks and deleted accounts, and files left by a crash.

### Quota and limits

Platform settings gain:

- `archives_enabled`: the admin switch
- `max_archive_bytes`: per-account archive storage. Null means unlimited; the hosted beta sets 500 MB.
- `max_archive_item_bytes`: the item cap, default 25 MB

Usage is the sum of `done` copies on the bookmarks a person owns. A manual archive over quota is refused with the space left. An automatic one is marked failed with "Archive storage is full". Settings shows usage beside the other quotas.

## Serving copies safely

An archived page is untrusted HTML. Scripts are stripped at capture, and the response is locked down as well, in case stripping misses something:

- HTML is served with `Content-Security-Policy: sandbox; default-src 'none'; img-src data:; style-src 'unsafe-inline' data:; font-src data:; media-src data:; frame-ancestors 'self'`. The sandbox without `allow-same-origin` gives the copy an opaque origin, so it cannot read the session or call the API.
- The app's own CSP gains `frame-src 'self'`, so the viewer and Quick peek can frame copies.
- PDFs and images are served inline with `nosniff`. SVG and any other type are served only as attachments.
- `?download=1` serves any copy as an attachment named `<host>-<YYYY-MM-DD>.<ext>`.
- A vault copy is served only while its vault is unlocked.
- Public list pages (`/l/{slug}`) do not expose copies.

## API

- `POST /bookmarks/{id}/archive` with `{ "formats": ["html", "pdf"] }`: queue an archive. Owner or write access. Answers with the archive status.
- `POST /bookmarks/archive` with `{ "ids": [...], "formats": [...] }`: bulk; reports per-bookmark failures like the other bulk actions.
- `GET /bookmarks/{id}/archive`: status per format.
- `GET /bookmarks/{id}/archive/{format}`: the copy, with `?download=1` for an attachment. Read access.
- `DELETE /bookmarks/{id}/archive`: delete every copy. Owner or write access.
- `BookmarkView` gains an `archive` summary (formats kept, capture time, latest status) for badges and menus.
- User settings gain `archive_new` (bool) and `archive_formats`. Quota usage gains `archive_bytes`.

## Interface

- **Bookmark menu:** **Archive…** opens a dialog with HTML (checked) and PDF. When a copy exists it warns: "This replaces the copy from 3 Nov 2026." **Open archive** appears once a copy exists.
- **Viewer** (`/archive/{id}`): a Linkdean bar with the capture date, a link to the live page and download buttons, above the copy in a sandboxed frame. A direct file opens in the browser's own viewer, or downloads.
- **Bookmark drawer:** an Archive section with the status (queued, archiving, or the failure reason), capture date and size, and Open, Download, Archive again and Delete actions.
- **Cards and rows:** a small archive icon when a copy exists. A bookmark marked broken by the link checker offers **Open archived copy**.
- **Quick peek:** when a site refuses to be framed, peek shows the copy if there is one, else the preview image as today.
- **Bulk bar:** **Archive** with the same dialog.
- **Settings → Archive:** the automatic switch, default formats, and storage used against the quota.
- **Admin → Platform settings:** enable archiving, per-account quota, item cap.

## Deploy and operations

- An `archives` volume in the production and dev Compose files, mounted at `ARCHIVE_DIR`.
- New environment variables `ARCHIVE_DIR` and `ARCHIVE_CONCURRENCY`, documented in `self-hosting/environment.md`.
- The hosted beta's backup (restic, in the private infrastructure repository) includes the archives volume. That is a separate change there.
- Self-hosters turn archiving off or cap it in Admin → Quotas.

## Testing

- Converter unit tests on fixture MHTML files: images and CSS `url()` inlined, scripts and handlers removed, oversized resources dropped, the item cap enforced.
- Database tests: the queue (claim, retry after restart), quota refusal, access by owner, write and read invitees, vault encryption across moves, deletion on purge.
- API tests: response headers for each format, download names, access refused without read access, vault copies refused while locked.
- Web tests: the archive dialog and its overwrite warning, badges, the drawer section, the Quick peek fallback.
- An ignored integration test that captures a real page through Chrome on the smoke stack.

## Later

- Version history: keep earlier copies.
- Embedded media (follow-up): Chrome's MHTML already includes plain `<video>` / `<audio>` / `<source>` files it fetched; the converter inlines those as `data:` under the per-resource cap (same as images). Still open: fetching media URLs that never landed in the snapshot (lazy players, separate ranges), while skipping streams (`m3u8`, `blob:`, MSE). Stay inside the item and account caps; fail soft and leave the live link.
