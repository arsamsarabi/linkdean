# Features

What Linkdean can do today. For planned work see [ROADMAP.md](./ROADMAP.md). For stack and domain model see [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## Library

- Nested categories (folders) and tags
- Optional category icons (large curated Tabler set; picker has search + scroll + pagination)
- Edit category drawer (name, parent, icon, hidden) from folder menu
- Bookmarks with title, URL, description, notes
- Bookmark cards / glance tiles: title primary, full URL muted beneath
- Custom bookmark preview image (upload + 16:9 crop; replaces screenshot/OG on the card; removable)
- Multi-category filing (a bookmark can live in more than one folder)
- Grid, compact list and explorer library views (preference persisted). The list shows each bookmark on one row: site icon, title, site and description, folders, badges and the same menu
- Unsorted / uncategorized filter
- Library filters: category, tag, created-within (7d/30d/90d/year), vault-only
- Hidden categories (declutter without encrypting)
- Soft-delete with trash: restore, purge, retention days, optional show-deleted-in-library
- Bulk select: move, tag, delete. Tick a bookmark's box (shown on hover or focus) to start selecting without pressing Select; shift-tick selects the range between; the bulk bar stays under the header while you scroll. Select stays for touch screens
- Duplicate detection and merge UI (sidebar → Library health → Duplicates)
- Broken links in the sidebar's Library health group, opening a drawer where each link can be opened, edited, rechecked or removed (Remove bookmark moves it to Deleted items); red badges on confirmed dead / unreachable cards
- Duplicate / fork a category (copy folder + bookmarks)
- Reading list (unread), favorites, and pin-to-top; opening a link can mark unread as read
- Category manager in Settings (explorer + create drawer)
- Command palette (mod+K, from the header search field): add, search, jump category
- `/` focuses the library search field on the Library and Vaults pages

---

## Search & enrichment

- Meilisearch full-text search for **non-vault** bookmarks
- Shared-folder visibility in search (owned + readable shared categories)
- Wait/retry UX when Meili lags writes: local matches merged + retry banner
- Favicon and OG image cached on disk (`PREVIEW_DIR`); `/favicon` and `/og` serve cached bytes
- Optional headless Chrome screenshots for previews (a removed preview is not retaken until the URL changes; a failed screenshot waits a week before the boot backfill retries it)
- Admin reindex: rebuild Meili from Postgres (Settings → Admin)
- Broken-link checker (in-process rate-limited worker): HEAD→GET probe, per-host pacing, private-host skip; vault excluded; confirmed after 2 soft fails or immediate 404/410; sidebar Library health entry → drawer + card badges; `POST /bookmarks/{id}/check-link`

---

## Vault & privacy

- Multiple named vaults, each with its own passphrase and key (Argon2id + AES-GCM)
- Vaults hold categories and loose bookmarks; move items in, out and between vaults, with a checklist when something blocks the move
- Recovery key per vault, shown once; replacing it asks for the vault passphrase. A lost passphrase and recovery key means the contents are gone
- Optional global passphrase that unlocks chosen vaults in one step, alongside each vault's own passphrase; turning it off asks for it
- Vaults page: open vaults get the Library's filters, sorting, views and bulk actions, with a vault filter and lock controls; the sidebar link shows only while a vault is open
- Settings for whether vault items also appear in the Library, and whether deleted vault items go to Trash or are removed immediately
- Unlock from Settings or the command palette's Unlock vault command; locking clears decrypted data from the browser cache
- Vault content never indexed in Meilisearch
- Export never dumps vault bookmarks as plaintext
- Unlocking a vault opens it for that login session only. Other devices, other sessions and API keys stay locked
- Optional auto-lock after inactivity (Settings). The server enforces it too: unlocked keys it has not used for that long are dropped. Logging out, a password change or a ban locks them

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
- Pending email invites: share with an address that has no account yet; the invite waits for them to register, and access starts when they accept
- Accept or decline shares from the notifications drawer; nothing appears in your library until you accept. Declining is silent and stops repeat invites until you restore it from Profile
- Collaborative edit on write-shared collections
- Save a shared bookmark into your own library (⋮ → Save to my library; lands in Unsorted)
- Share dashboards: read / write; invitees see shared boards in the switcher
- RW invitees can rearrange shared dashboards per ACL
- Pin a shared dashboard as your default; losing access returns you to your own default
- Share badges on cards, glance tiles, folders, and dashboard chrome (invitee + outgoing)
- Meili visibility respects readable shared categories
- Read-only library gates for invitees
- Vault pins stay owner-only on shared dashboards
- **Public bookmark list URLs:** owner enables unlisted `/l/{slug}` link (Share drawer); anyone with the URL can read the list; rotate/revoke; vault lists blocked; notes never exposed

---

## Shared activity & social

- In-app notifications: share invite / access change / revoke, shared edits
- Bell + drawer in the app shell (unread count)
- Likes, comments, and 1–5 ratings on **shared** bookmarks, lists, and dashboards
- Follow / unfollow users (⋮ on shared bookmarks; lists on Profile; notifies the followee)
- Block / unblock users (⋮ on shared bookmarks; list on Profile): removes follows and shares both ways without telling them, and hides each other's social activity and notifications
- Profiles (`/u/{id}`): avatar, name, bio, follower and following lists, follow button, and the bookmarks the owner chose to show. Your own profile adds email and an edit drawer
- Visibility: a profile is Hidden, Followers or Members; a category is Only me, Followers or Members; a bookmark can be Only me. Everything starts private. Members means anyone signed in, and Public links stay the only access without an account
- Mute / unmute users (⋮ on shared bookmarks; list on Profile): one-way quiet, hides their social activity and notifications from you
- Owner notified on social activity (API `/social/…`)

---

## Personal activity history

- Per-user create / edit / delete / restore log for bookmarks, lists, and dashboards
- History drawer, opened from the sidebar's Library health group (filter by subject + action)
- Distinct from shared-activity notifications; vault bookmark titles redacted
- API `GET /activity`

---

## Auth & accounts

- Local email / password accounts
- Optional OIDC sign-in with any standard issuer (discovery, PKCE, nonce and ID token checks)
- First registered user becomes admin (bootstrap)
- Open registration toggle (Admin)
- Admin registration invites: one-time `ldi_…` codes / copyable `/register?invite=` links (optional email bind); required when registration closed
- Optional `BOOTSTRAP_TOKEN`: the first account (the admin) must enter it on the setup page. Racing sign-ups cannot make two first admins or redeem one invite twice
- Profile: display name, bio, avatar, password change. A new password signs out every other session, and can revoke all API keys too
- Admin accounts list: promote / demote admin (last admin protected), per-user API-keys gate
- Session auth; multi-device via server account
- Admin bans with a reason, for N days, until a date or permanently; a banned user sees why on `/banned`, and API keys stop working while the ban lasts
- Banned users list with bulk unban and bulk change of reason or duration
- Moderation of a banned user's exposed content: turn off public links, set items private, hide the profile, revoke shares, delete comments, move items to the owner's Trash; private and vault content stay out of reach, and every action is recorded

---

## API keys

- Users create and revoke keys scoped to their own data (Settings → API keys)
- Bearer `ldk_…` accepted alongside session
- Secret hashed at rest; plaintext shown once at creation
- Platform-wide and per-user enable/disable (Admin)

---

## Import / export

- Import: Netscape HTML, Linkdean JSON, Pocket CSV, Raindrop CSV
- Imported bookmarks get metadata and previews in the background
- Export: Netscape HTML, Linkdean JSON
- Linkdean JSON (version 2) keeps every folder of a bookmark, its favourite, unread, pinned and private flags, its created and updated times, and your dashboards. Importing adds the dashboards beside your own
- Netscape HTML lists a bookmark under each folder it is in, and reads `<DD>` descriptions on import

---

## Ops & self-host

- Docker Compose stack (API, Postgres, Meilisearch, optional Chrome)
- Healthchecks on db / Meili / api; `GET /api/health` and `GET /api/ready`
- Prometheus scrape: `GET /api/metrics` (HTTP counters / latency histograms by route template + process metrics), behind `METRICS_TOKEN`
- Outbound email over SMTP (optional): an outbox with retries, Settings → Admin → Email with status, delivery counts and a test send; Mailpit catches dev email and `/api/dev/emails` previews templates
- Avatars and preview volumes; backup recipe in [SELF_HOSTED.md](./SELF_HOSTED.md)
- Sidebar navigation (Dashboard, Library, Vaults, quick views, the folder tree with Unsorted, Library health, Deleted items, settings); on phones the burger slides it in. On wide screens the header's sidebar button (or ⌘B / Ctrl+B) hides and shows it, remembered per browser
- Shared page (sidebar → Shared, with a count of invites waiting): accept or decline invites, open folders and dashboards others share with you, and see what you share and with whom; `/?board=<id>` opens a given dashboard
- Config options: sections (General, Categories, Import / Export, Vault, API keys, Admin) listed down the side on wide screens and as a scrolling row on phones; `?tab=` opens one
- Sidebar Folders section collapses: collapsed it shows Unsorted, then the 3 folders with the latest activity (a bookmark added or changed in them, or the folder itself made or renamed) and the open folder; expanded it shows the whole tree with a filter field (matches at any depth, with their path). The choice is remembered per browser
- Phone tab bar: Home, Library, Add (into the open folder), Search (command palette) and Me; the burger keeps the full sidebar
- An open folder titles the library page, with a breadcrumb back to the whole library (Unsorted included); an empty folder offers Add bookmark here
- The page, drawers and modals keep their scrollbar space reserved, so content does not shift sideways when a scrollbar appears or a panel opens
- Header search field opens the command palette (which can also add a bookmark); the library page's Add bookmark button, the phone tab bar's Add and paste-to-add open the new-bookmark form
- Editing a shared bookmark: the side panel has Details and Discussion tabs, so likes, rating and comments sit next to the edit form (the card menu's Likes & comments still opens them on their own)
- Paste a web address anywhere outside a text field to open the new-bookmark form with it filled in (in the open folder when on the library)
- PWA install: web manifest + service worker (standalone / “Add to Home Screen”)
- Hardened before release: race-safe sign-up, invites and admin demotion; vault moves lock their rows; the browser session lives only in the `HttpOnly` cookie, with a client header guarding writes; Chrome runs sandboxed, non-root and off the data network; a `.dockerignore` keeps local files out of the image

---

## Surfaces

| Surface | Role |
| --- | --- |
| `/` | Dashboard (glance boards) |
| `/library` | Bookmark library |
| `/settings` | Preferences, categories, import, vault, API keys, admin |
| `/vaults` | Open vaults, with Library filters and views |
| `/banned` | Reason and end date for a banned account |
| `/trash` | Soft-deleted items |
| `/profile` | Account profile |
| Command palette | Quick add / search / navigate |
