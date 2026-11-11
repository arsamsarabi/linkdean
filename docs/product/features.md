# Features

What Linkdean can do today. For planned work see [roadmap.md](./roadmap.md). For stack and domain model see [architecture.md](./architecture.md).

---

## Library

- Nested categories (folders) and tags
- Optional category icons (large curated Tabler set; picker has search + scroll + pagination; the chosen icon is filled and named beside the label)
- Edit category drawer (name, parent, icon, hidden) from folder menu
- Bookmarks with title, URL, description, notes
- Only `http://` and `https://` addresses are saved, on create, edit and import; an address typed without a scheme gets `https://`
- **Import from URL** on the bookmark form: the server fetches the page (plain HTTP, public addresses only) and fills in the title, description and tags the person has left empty. Not offered for bookmarks going into a vault. Counts as one enrich action and is limited to 60 per user every 15 minutes
- **Quick save** at `/save?url=…&title=…&text=…`: opens the bookmark form filled from the link, fetches page details when needed, remembers the last folder, and offers **Save to Unsorted**. A URL already in the library opens that bookmark instead of a duplicate
- **Fast capture** — browser extension, bookmarklet, share target, and iOS Shortcuts: how to set each up is in [Saving pages fast](../guides/fast-capture.md)
- Bookmark cards / glance tiles: title primary, full URL muted beneath
- Custom bookmark preview image (upload + 16:9 crop; replaces screenshot/OG on the card; removable)
- Multi-category filing (a bookmark can live in more than one folder)
- Grid, compact list and explorer library views (preference persisted). The list shows each bookmark on one row: site icon, title, site and description, folders, badges and the same menu
- Library list is keyset-paginated (infinite scroll); folder, tag, flag, created-within, vault-only and sort run on the server with a total match count
- Unsorted / uncategorized filter
- Library filters: category, tag, created-within (7d/30d/90d/year), vault-only
- Hidden categories (declutter without encrypting)
- Soft-delete with trash: restore, purge, retention days, optional show-deleted-in-library
- Bulk select: move, tag, delete. Tick a bookmark's box (shown on hover or focus) to start selecting without pressing Select; shift-tick selects the range between; the bulk bar stays under the header while you scroll. Select stays for touch screens. Select all asks the server for every id the current filter matches (not only loaded rows)
- Duplicate detection and merge page (`/duplicates`, sidebar → Library health → Duplicates)
- Broken links page (`/broken-links`, sidebar → Library health), where each link can be opened, rechecked or removed (Remove bookmark moves it to Deleted items); red badges on confirmed dead / unreachable cards
- Duplicate / fork a category (copy folder + bookmarks)
- Reading list (unread), favorites, and pin-to-top; opening a link can mark unread as read
- Categories page (`/categories`, in the sidebar under Shared): the full tree with a name filter beside the chosen category's details, path, badges, subcategories and every action (open in library, edit, new subcategory, share, duplicate, move to vault, likes & comments, delete); `?id=` opens one, and phones show the tree or the details one at a time
- Command palette (mod+K, from the header search field): add, search, jump category
- `/` focuses the library search field on the Library and Vaults pages

---

## Search & enrichment

- Meilisearch full-text search for **non-vault** bookmarks
- Shared-folder visibility in search (owned + readable shared categories)
- Favicon and OG image cached on disk (`PREVIEW_DIR`); `/favicon` and `/og` serve cached bytes
- Optional headless Chrome screenshots for previews (a removed preview is not retaken until the URL changes; a failed screenshot waits a week before the boot backfill retries it)
- Admin reindex: rebuild Meili from Postgres (Admin)
- Broken-link checker (in-process rate-limited worker): HEAD→GET probe, per-host pacing, private-host skip; vault excluded; confirmed after 2 soft fails or immediate 404/410; sidebar Library health entry → page + card badges; `POST /bookmarks/{id}/check-link`

---

## Vault & privacy

- Multiple named vaults, each with its own passphrase and key (Argon2id + AES-GCM)
- Vaults hold categories and loose bookmarks; move items in, out and between vaults, with a checklist when something blocks the move
- Recovery key per vault, shown once; replacing it asks for the vault passphrase. A lost passphrase and recovery key means the contents are gone
- Optional global passphrase that unlocks chosen vaults in one step, alongside each vault's own passphrase; turning it off asks for it
- Vault search skips bookmarks in Deleted items
- Vaults page: open vaults get the Library's filters, sorting, views and bulk actions, with a vault filter and lock controls; the sidebar link shows only while a vault is open
- Settings for whether vault items also appear in the Library, and whether deleted vault items go to Trash or are removed immediately
- Unlock from Settings or the command palette's Unlock vault command; locking clears decrypted data from the browser cache
- Vault content never indexed in Meilisearch
- Export includes vault bookmarks as plaintext only while the vault is unlocked for that session; locked vaults stay out of the file
- Unlocking a vault opens it for that login session only. Other devices, other sessions and API keys stay locked
- Optional auto-lock after inactivity (Settings). The server enforces it too: unlocked keys it has not used for that long are dropped. Logging out, a password change or reset, or a ban locks them

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
- Pending email invites: share with an address that has no account yet, or whose account has not verified it; the invite waits until an account verifies the address, and access starts when they accept
- Invite claim links also work while signed in, for the account whose address the invite was sent to
- Invite emails (needs mail): sharing with a verified account emails it an invite to open Linkdean, and sharing with an address emails the claim link. The share drawer says "We emailed them an invite." For an address the owner still sees the claim link to copy. Registering or claiming through the emailed link also verifies the address; the owner's copied link does not. No email for a declined invitee or an access change. Each owner can send 20 invite emails an hour; past that the share is still saved and the drawer says no email was sent
- Accept or decline shares from the notifications drawer; nothing appears in your library until you accept. Declining is silent and stops repeat invites until you restore it from Profile. The owner keeps seeing a declined invite as pending and can still change or cancel it; a cancelled invite can no longer be restored or accepted
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
- Notifications drawer groups items by day (Today, Yesterday, then the date), gives each kind its own icon and colour, tints unread items with a dot, and shows an "all caught up" state when empty
- Bell + drawer in the app shell (unread count)
- Notification emails (needs mail): **Settings → Notifications** has an email switch per kind. Share invites and moderation changes are on by default; access changes, revoked shares, shared edits, likes, comments, ratings and new followers are off. The same block and mute rules apply, and each email links back to that settings tab. With mail off the tab says email needs outbound mail and the switches are disabled
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
- History page (`/history`), linked from the sidebar's Library health group (filter by subject + action)
- Distinct from shared-activity notifications; vault bookmark titles redacted
- API `GET /activity`

---

## Auth & accounts

- Local email / password accounts
- Optional OIDC sign-in with any standard issuer (discovery, PKCE, nonce and ID token checks)
- Single sign-on linking: an OIDC sign-in whose address belongs to a password account asks for that account's password once, then connects; the account gets a "Single sign-on was connected" email. **Profile → Edit profile → Single sign-on** shows the connected provider, with **Connect** (any provider address) and **Disconnect** (needs the password; not offered to SSO-only accounts)
- First registered user becomes admin (bootstrap)
- Open registration toggle (Admin); public beta keeps registration open and relies on the protections below
- Admin registration invites: one-time `ldi_…` codes / copyable `/register?invite=` links (optional email bind); required when registration closed
- Optional Cloudflare Turnstile on password registration (`TURNSTILE_SITE_KEY` / `TURNSTILE_SECRET_KEY`); off when unset; OIDC uncapped by Turnstile
- Optional disposable-email domain block at register and email-change (`BLOCK_DISPOSABLE_EMAILS`)
- Per-user quotas (Admin platform settings; null = unlimited): max bookmarks, stored image bytes, enrich actions per UTC day, import size, upload image size (clamped to hard caps). Usage shown in Settings when a limit is set
- Optional `BOOTSTRAP_TOKEN`: the first account (the admin) must enter it on the setup page. Racing sign-ups cannot make two first admins or redeem one invite twice
- Profile: display name, bio, avatar, password change, email change. A new password signs out every other session, and can revoke all API keys too
- Password reset: with mail on, **Forgot password?** emails a single-use link that works for 30 minutes; admins can make a 24-hour link to hand over, the only way back in when mail is off. A reset signs out every session, can revoke all API keys, and leaves vaults locked
- Email verification (needs mail): a password registration emails a 24-hour, single-use link, and unverified accounts see a banner with **Resend email**. Opening the link verifies the address without signing in. OIDC accounts are verified when the provider reports the address as verified. Accounts that existed before verification shipped count as verified
- Email change: **Profile → Edit profile → Change email** with the current password sends a link to the new address and a notice to the old one; the address changes only when the link is opened. Not for SSO-only accounts; with mail off it says to ask an admin
- Admin verification controls: an **Unverified** badge in the accounts list, **Mark verified**, **Remove verification**, and **Change email** applied at once
- **Require verified email** (Admin, needs mail): unverified accounts can only verify or sign out until they do. Admins are never held. Host env `REQUIRE_VERIFIED_EMAIL=true` forces the same gate and locks the admin toggle
- Admin accounts list: promote / demote admin (last admin protected), per-user API-keys gate
- Session auth; multi-device via server account
- Delete account (Settings → General): confirm email (and password when the account has one), 7-day grace with cancel, then hard purge of owned data including shared folders; last admin cannot schedule
- Admin bans with a reason, for N days, until a date or permanently; a banned user sees why on `/banned`, and API keys stop working while the ban lasts
- Legal pages (draft, not counsel-reviewed): Terms of Service, Privacy Policy (necessary cookies only, no consent banner) and Acceptable Use Policy at `/terms`, `/privacy` and `/acceptable-use`, linked from sign-in and sign-up
- Terms at sign-up: password sign-up and first-admin setup need a terms checkbox and the API refuses a missing or stale `terms_version`; SSO sign-in needs the checkbox too (`/auth/oidc/start?terms_version=`), and a new SSO account records it. The version and time are stored on the user (`users.terms_version`, `terms_accepted_at`); accounts from before are NULL and are not re-prompted
- Beta banner on every page (no uptime promise, export regularly), dismissible for the browser session
- Report abuse (signed-in users): a **Report** control on public lists (`/l/…`, sign-in link when signed out), other people's profiles and other people's comments, with a reason (spam, harassment, illegal, impersonation, other) and optional details. Reporting your own content is refused; a second report of the same target while yours is open returns the first; 10 reports per 15 minutes per user
- Admin report queue (Admin → Abuse reports): open reports with reporter, reason and details. **Dismiss** closes it; **Hide content** revokes the public link, hides the profile or deletes the comment (recorded in the user's moderation history); **Warn** records a note and emails the user when mail is on; **Ban** bans the user with the note (or the report reason) and still resolves the report if they are already banned
- Abuse contact: admins set an email in the same panel (`platform_settings.abuse_contact_email`); the report form shows it. Terms and Privacy tell people to use Report or that address. Marketing site footer also lists `contact@linkdean.app`
- Banned users list with bulk unban and bulk change of reason or duration
- A banned user's public lists stop loading for anyone while the ban lasts, and come back when it ends or is lifted
- Moderation of a banned user's exposed content: turn off public links, set items private, hide the profile, revoke shares, delete comments, move items to the owner's Trash; private and vault content stay out of reach, and every action is recorded

---

## API keys

- Users create and revoke keys scoped to their own data (Settings → API keys)
- Bearer `ldk_…` accepted alongside session
- A key cannot create more keys or use admin routes, even an admin's, so revoking a leaked key ends its access
- Secret hashed at rest; plaintext shown once at creation
- Platform-wide and per-user enable/disable (Admin)
- **Browser extension sign-in** without handling a key: the extension opens `/connect/extension` in a window of its own, the person signs in as usual and chooses **Allow**, the page hands a single-use code to the extension's script on that page, and the extension trades it (two minutes, PKCE S256, bound to the extension's `chrome-extension://` or `moz-extension://` address) at `POST /api/auth/extension/token` for a key named "Browser extension · Chrome" or "· Firefox". Approving needs a login session, never a key. Signing out in the extension revokes its key. Needs API keys enabled for the account

---

## Import / export

- Import: Netscape HTML, Linkdean JSON, Pocket CSV, Raindrop CSV
- Imported bookmarks get metadata and previews in the background
- Export: Netscape HTML, Linkdean JSON (full account export of owned folders, bookmarks and dashboards; vault items included while unlocked)
- Linkdean JSON (version 2) keeps every folder of a bookmark, its favourite, unread, pinned and private flags, its created and updated times, and your dashboards. Importing adds the dashboards beside your own
- Netscape HTML lists a bookmark under each folder it is in, and reads `<DD>` descriptions on import
- One import at a time per user, and at most 10 per 15 minutes. A file may hold up to 10,000 folders (Raindrop CSV included), 100 dashboards and 100 sections per dashboard

---

## Ops & self-host

- Docker Compose stack (API, Postgres, Meilisearch, optional Chrome)
- Release images: a `v*` tag runs CI, publishes `ghcr.io/arsamsarabi/linkdean:<version>` and creates a GitHub release; [quick-start](../self-hosting/quick-start.md) has the Compose file and optional Chrome setup, and [environment](../self-hosting/environment.md) is the full variable reference (also on [docs.linkdean.app](https://docs.linkdean.app/docs/self-hosting/quick-start))
- Public docs site: [docs.linkdean.app](https://docs.linkdean.app) (Docusaurus in `apps/docs`, content synced from this `docs/` tree)
- Public marketing site: [linkdean.app](https://linkdean.app) (Astro in `apps/site`)
- Public app: [app.linkdean.app](https://app.linkdean.app) (homelab beta stack in `deploy/public-beta/`)
- Healthchecks on db / Meili / api; `GET /api/health` (reports the running version) and `GET /api/ready`
- Security headers on every response (`Content-Security-Policy`, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`, `X-Frame-Options`; optional `Strict-Transport-Security` via `HSTS_MAX_AGE`). Image routes keep their own sandbox CSP. `/.well-known/security.txt` and `/security.txt` point to the project security contact
- Supply chain: CI runs `cargo audit` and `pnpm audit --prod` on every PR; Dependabot opens updates for Cargo, npm and GitHub Actions
- Chrome on an internal network with no route out: it reaches the web only through the API's egress proxy, which resolves each host itself and refuses private addresses
- Public beta operations (`deploy/public-beta/`): restic backups to B2 with a scripted restore drill into a throwaway, network-isolated stack ([BACKUP.md](https://github.com/arsamsarabi/linkdean/blob/main/deploy/public-beta/BACKUP.md)); external uptime check list ([MONITORING.md](https://github.com/arsamsarabi/linkdean/blob/main/deploy/public-beta/MONITORING.md)); a sign-up controls check that creates no account (`scripts/check-signup-controls.sh`)
- Prometheus scrape: `GET /api/metrics` (HTTP counters / latency histograms by route template + process metrics), behind `METRICS_TOKEN`
- Structured logs: `LOG_FORMAT=text|json` on stdout (default text); optional `LOG_FILE` appends JSONL; every response has `X-Request-Id` correlated into request log spans
- Outbound email over SMTP (optional): an outbox with retries, Admin → Email with status, delivery counts and a test send; Mailpit catches dev email and `/api/dev/emails` previews templates
- Avatars and preview volumes; backup recipe in [../self-hosting/operations.md](../self-hosting/operations.md)
- Sidebar navigation (Dashboard, Library, Vaults, quick views, the folder tree with Unsorted, Library health, Deleted items, settings); on phones the burger slides it in. On wide screens the header's sidebar button (or ⌘B / Ctrl+B) hides and shows it, remembered per browser
- Shared page (sidebar → Shared, with a count of invites waiting): accept or decline invites, open folders and dashboards others share with you, and see what you share and with whom; `/?board=<id>` opens a given dashboard
- Config options: sections (General, Notifications, Fast capture, Import / Export, Vault, API keys) listed down the side on wide screens and as a scrolling row on phones; `?tab=` opens one
- Sidebar Folders section collapses: collapsed it shows Unsorted, then the 3 folders with the latest activity (a bookmark added or changed in them, or the folder itself made or renamed) and the open folder; expanded it shows the whole tree with a filter field (matches at any depth, with their path). The choice is remembered per browser
- Phone tab bar: Home, Library, Add (into the open folder), Search (command palette) and Me; the burger keeps the full sidebar
- An open folder titles the library page, with a breadcrumb back to the whole library (Unsorted included); an empty folder offers Add bookmark here
- On systems whose scrollbars take up space, the page, drawers and modals scroll under a thin overlay scrollbar instead, so there is no empty gutter and content does not shift sideways when a scrollbar appears or a panel opens
- Header search field opens the command palette (which can also add a bookmark); the library page's Add bookmark button, the phone tab bar's Add and paste-to-add open the new-bookmark form
- Editing a shared bookmark: the side panel has Details and Discussion tabs, so likes, rating and comments sit next to the edit form (the card menu's Likes & comments still opens them on their own)
- Paste a web address anywhere outside a text field to open the new-bookmark form with it filled in (in the open folder when on the library)
- PWA install: web manifest + service worker (standalone / “Add to Home Screen”)
- Hardened before release: race-safe sign-up, invites and admin demotion; vault moves lock their rows; the browser session lives only in the `HttpOnly` cookie, with a client header guarding writes; Chrome runs sandboxed, non-root and off the data network; a `.dockerignore` keeps local files out of the image

---

## Surfaces

- Mascot (Dean) on moment screens only: empty library / folder / search, empty Deleted, fully empty Shared, 404, onboarding, and the wide sign-in panel. Soft light backdrop; decorative (`alt=""`). Not in header, sidebar, cards, dashboards, or settings.
- Drawers go full-width on phones (`sm` and up keep `md`/`lg`); Deleted items table scrolls horizontally; app shell padding tightens on small screens; dashboard glance tiles stack to one column on the narrowest phones.

| Surface | Role |
| --- | --- |
| `/` | Dashboard (glance boards) |
| `/library` | Bookmark library |
| `/settings` | Preferences, categories, import, vault, API keys, admin |
| `/vaults` | Open vaults, with Library filters and views |
| `/banned` | Reason and end date for a banned account |
| `/terms`, `/privacy`, `/acceptable-use` | Legal pages (public) |
| `/trash` | Soft-deleted items |
| `/profile` | Account profile |
| Command palette | Quick add / search / navigate |
| [linkdean.app](https://linkdean.app) | Marketing (static) |
| [docs.linkdean.app](https://docs.linkdean.app) | Documentation (static) |
| [app.linkdean.app](https://app.linkdean.app) | Hosted app (public beta) |
