# Features

What Linkdean can do today. For planned work see [ROADMAP.md](./ROADMAP.md). For stack and domain model see [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## Library

- Nested categories (folders) and tags
- Optional category icons (picker on create / edit)
- Edit category drawer (name, parent, icon, hidden) from folder menu
- Bookmarks with title, URL, description, notes
- Bookmark cards / glance tiles: title primary, full URL muted beneath
- Multi-category filing (a bookmark can live in more than one folder)
- Grid and explorer library views (preference persisted)
- Unsorted / uncategorized filter
- Library filters: category, tag, created-within (7d/30d/90d/year), vault-only
- Hidden categories (declutter without encrypting)
- Soft-delete with trash: restore, purge, retention days, optional show-deleted-in-library
- Bulk select: move, tag, delete
- Duplicate detection and merge UI
- Duplicate / fork a category (copy folder + bookmarks)
- Reading list (unread), favorites, and pin-to-top; opening a link can mark unread as read
- Category manager in Settings (explorer + create drawer)
- Command palette (`/` or mod+K): add, search, jump category

---

## Search & enrichment

- Meilisearch full-text search for **non-vault** bookmarks
- Shared-folder visibility in search (owned + readable shared categories)
- Wait/retry UX when Meili lags writes: local matches merged + retry banner
- Favicon and OG image cached on disk (`PREVIEW_DIR`); `/favicon` and `/og` serve cached bytes
- Optional headless Chrome screenshots for previews
- Admin reindex: rebuild Meili from Postgres (Settings → Admin)

---

## Vault & privacy

- Password-protected vault categories (Argon2id + AES-GCM)
- Vault content never indexed in Meilisearch
- Export never dumps vault bookmarks as plaintext
- Vault unlock in session; optional auto-lock after inactivity (Settings)

---

## Dashboards

- Home at `/`; library at `/library`
- Sectioned glance grid of pinned bookmarks
- Multi-dashboard: create, rename, pin default, delete (fallback / recreate Home)
- Page switcher; Settings default-dashboard select
- `default_landing` `/` vs `/library` (Settings + post-login; logo honors it)

---

## Sharing (same instance)

- Share folders with invitees: **read** or **write** ACL
- Collaborative edit on write-shared collections
- Share dashboards: read / write; invitees see shared boards in the switcher
- RW invitees can rearrange shared dashboards per ACL
- Share badges on cards, glance tiles, folders, and dashboard chrome (invitee + outgoing)
- Meili visibility respects readable shared categories
- Read-only library gates for invitees
- Vault pins stay owner-only on shared dashboards

---

## Shared activity & social

- In-app notifications: share invite / access change / revoke, shared edits
- Bell + drawer in the app shell (unread count)
- Likes, comments, and 1–5 ratings on **shared** bookmarks, lists, and dashboards
- Owner notified on social activity (API `/social/…`)

---

## Personal activity history

- Per-user create / edit / delete / restore log for bookmarks, lists, and dashboards
- History drawer in the app shell (filter by subject + action)
- Distinct from shared-activity notifications; vault bookmark titles redacted
- API `GET /activity`

---

## Auth & accounts

- Local email / password accounts
- Optional OIDC (e.g. Authentik-shaped issuers)
- First registered user becomes admin (bootstrap)
- Open registration toggle (Admin)
- Admin registration invites: one-time `ldi_…` codes / copyable `/register?invite=` links (optional email bind); required when registration closed
- Profile: display name, bio, avatar, password change
- Admin accounts list: promote / demote admin (last admin protected), per-user API-keys gate
- Session auth; multi-device via server account

---

## API keys

- Users create and revoke keys scoped to their own data (Settings → API keys)
- Bearer `ldk_…` accepted alongside session
- Secret hashed at rest; plaintext shown once at creation
- Platform-wide and per-user enable/disable (Admin)

---

## Import / export

- Import: Netscape HTML, Linkdean JSON, Pocket CSV, Raindrop CSV
- Export: Netscape HTML, Linkdean JSON

---

## Ops & self-host

- Docker Compose stack (API, Postgres, Meilisearch, optional Chrome)
- Healthchecks on db / Meili / api; `GET /api/health` and `GET /api/ready`
- Prometheus scrape: `GET /api/metrics` (HTTP counters / latency histograms + process metrics)
- Avatars and preview volumes; backup recipe in [self-host.md](./self-host.md)
- Mobile burger nav for small viewports
- PWA install: web manifest + service worker (standalone / “Add to Home Screen”)

---

## Surfaces

| Surface | Role |
| --- | --- |
| `/` | Dashboard (glance boards) |
| `/library` | Bookmark library |
| `/settings` | Preferences, categories, import, vault, API keys, admin |
| `/trash` | Soft-deleted items |
| `/profile` | Account profile |
| Command palette | Quick add / search / navigate |
