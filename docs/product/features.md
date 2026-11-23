# Features

What Linkdean can do today. For planned work see [roadmap.md](./roadmap.md). For stack and domain model see [architecture.md](./architecture.md).

---

## Library

- Nested categories (folders) and tags. A comma separates tags, so it is never part of one
- Optional category icons (large curated Tabler set; picker has search + scroll + pagination; the chosen icon is filled and named beside the label)
- Edit category drawer (name, parent, icon, hidden) from folder menu
- Bookmarks with title, URL, description, notes
- Bookmark cards and list rows show a bookmark's folders right under the title as breadcrumbs (parent folders dimmed), always visible. Tags sit behind a **N tags** toggle, so long tag lists don't make cards taller: on cards they open under the toggle, on list rows in a popover
- Only `http://` and `https://` addresses are saved, on create, edit and import; an address typed without a scheme gets `https://`
- **Import from URL** on the bookmark form: the server fetches the page (plain HTTP, public addresses only) and replaces the form's title and description with the page's, adding its tags to the ones already there. A field edited while the page loads keeps the edit; the automatic fetch on the quick save page fills only empty fields. Not offered for bookmarks going into a vault. Counts as one enrich action and is limited to 60 per user every 15 minutes
- **Quick save** at `/save?url=…&title=…&text=…`: opens the bookmark form filled from the link, fetches page details when needed, remembers the last folder, and offers **Save to Unsorted**. A URL already in the library opens that bookmark instead of a duplicate
- **Fast capture** — browser extension, bookmarklet, share target, and iOS Shortcuts: how to set each up is in [Saving pages fast](../guides/fast-capture.md). **Config options → Fast capture** links to the extension on the [Chrome Web Store](https://chromewebstore.google.com/detail/ielbdodidimpcolkjhkjdaifpfibolhf) and [Firefox Add-ons](https://addons.mozilla.org/firefox/addon/linkdean/)
- Bookmark cards / glance tiles: title primary, full URL muted beneath
- **Quick peek**: open a bookmark inside Linkdean from a card, row or dashboard tile (Peek in the menu, or the eye control on tiles). The panel maximizes, minimizes, detaches, opens in a new tab, or closes (Esc); size is remembered per browser. Primary click still opens a new tab. Vault bookmarks and non-https addresses stay out. Pages that refuse to embed fall back to the preview, title, description and **Open in new tab**. Peeking counts as an open
- Custom bookmark preview image (upload + 16:9 crop; replaces screenshot/OG on the card; removable)
- Multi-category filing (a bookmark can live in more than one folder)
- Create a category from the bookmark form: typing a name that matches no category offers **Create "name"**, which creates it at the top level, selects it and toasts **Category created**. Nest under a parent from the Categories page. Not offered on read-only shared bookmarks
- Bookmark URL field: blur checks for a valid web address and blocks Add until it passes; the API also refuses empty hosts and hosts with spaces
- **Move to vault** with no vault yet shows **You haven't set up a vault yet** and a **Set up vault** link to `/vaults`
- Grid, compact list and explorer library views (preference persisted). The list shows each bookmark on one row: site icon, title, site and description, folders, badges and the same menu
- Library list is keyset-paginated (infinite scroll); folder, tag, flag, created-within, vault-only, sort and paging all run in one database query per page, so scrolling costs the same in a large library, with a total match count
- Explorer folder tiles show how many bookmarks are directly in each folder and in Unsorted, counted on the server
- Unsorted / uncategorized filter; the sidebar's Unsorted entry shows how many bookmarks wait there
- **Filing queue** (`/file`, or **File one by one** on Unsorted): steps through Unsorted newest first, one card at a time, to pick folders (or create one), add tags, fix the title and mark it a favorite. **File** needs a folder; **Skip** leaves the bookmark for later; **Delete** moves it to Deleted items. Folders and tags already used on the same site are one-click suggestions. Keys on desktop: 1–5 pick a suggested folder, Enter files, S skips, D deletes; on phones swipe left to skip, right to file
- Library filters: category, tag, created-within (7d/30d/90d/year), vault-only
- **Saved searches:** name the current library query and filters (Settings of the moment), reopen from the library toolbar or the sidebar **Saved searches** list; rename or delete anytime. Cap 50 per account. Design: [saved searches](../design/saved-searches.md)
- Hidden categories (declutter without encrypting)
- Soft-delete with trash: restore, purge, retention days, optional show-deleted-in-library
- Bulk select: move, tag, delete. Tick a bookmark's box (shown on hover or focus) to start selecting without pressing Select; shift-tick selects the range between; the bulk bar stays under the header while you scroll. Select stays for touch screens. Select all asks the server for every id the current filter matches (not only loaded rows). When some bookmarks fail, the notice says why and they stay selected; Move to vault takes only your own bookmarks
- Duplicate detection and merge page (`/duplicates`, sidebar → Library health → Duplicates)
- Broken links page (`/broken-links`, sidebar → Library health), where each link can be opened, rechecked or removed (Remove bookmark moves it to Deleted items); red badges on confirmed dead / unreachable cards
- Duplicate / fork a category (copy folder + bookmarks)
- Reading list (unread), favorites, and pin-to-top; opening a link can mark unread as read
- Categories page (`/categories`, in the sidebar under Shared): the full tree with a name filter beside the chosen category's details, path, badges, subcategories and every action (open in library, edit, new subcategory, share, duplicate, move to vault, likes & comments, delete); `?id=` opens one, and phones show the tree or the details one at a time
- Command palette (mod+K, from the header search field): add, search, jump category
- `/` focuses the library search field on the Library and Vaults pages

---

## Search & enrichment

- Meilisearch full-text search for **non-vault** bookmarks; category chips and tags on hits come from Postgres, with a catch-up reindex when Meili's copy lags
- Shared-folder visibility in search (owned + readable shared categories)
- Favicon and OG image cached on disk (`PREVIEW_DIR`); `/favicon` and `/og` serve cached bytes
- Optional headless Chrome screenshots for previews (a removed preview is not retaken until the URL changes; a failed screenshot is retried automatically once it is older than the admin's retry window, 7 days by default, by a check that runs at start and every 6 hours)
- Admin **Retry failed screenshots** (Admin → Screenshots): shows how many failed screenshots would be retried and queues them two at a time. Allowed once per cooldown (24 hours by default, 1 to 720) and takes only failures older than the cooldown; the button says when it is next available. The cooldown and the automatic retry window (1 to 90 days) are platform settings
- Admin reindex: rebuild Meili from Postgres (Admin → Search index)
- Broken-link checker (in-process rate-limited worker): HEAD→GET probe, per-host pacing, private-host skip; vault excluded; confirmed after 2 soft fails or immediate 404/410; sidebar Library health entry → page + card badges; `POST /bookmarks/{id}/check-link`

---

## Archive

- Archive a bookmark from its menu, the bookmark drawer or the Library's select mode (up to 500 at once): keep the page as single-file HTML (scripts removed; images, styles, fonts and plain video/audio files Chrome already fetched inlined), as a PDF, or both. An address that serves a file (PDF, image, audio, video) is kept as that file. Captures run in the background through the shared Chrome, two at a time by default (`ARCHIVE_CONCURRENCY`)
- **Archive new bookmarks** (Settings → Archive, off by default) archives each bookmark you save or import in the chosen formats
- Viewer at `/archive/{id}`: the copy inside Linkdean with no scripts and links opening in a new tab, a switch between formats, the live page and a download. Cards show an **archived** badge; a broken or unreachable badge opens the copy; Quick peek offers the copy when a site refuses to embed
- Archiving again replaces the copy and says so first; a failed attempt keeps the older copy. **Delete archive** removes every copy and keeps the bookmark
- Library search matches words from the body of non-vault HTML archives (extracted when the copy finishes). Vault bookmarks never contribute page text. Existing HTML copies gain body search on the next archive
- Downloads are named `<site>-<date>.<ext>`. HTML copies also offer a **Markdown** download (reader-mode headings and paragraphs, converted on the fly; vault while unlocked). Copies are served with a sandbox content security policy; an original file a browser could run (HTML, SVG) is only ever downloaded
- Vault bookmarks: copies are encrypted with the vault key, captured and opened only while the vault is unlocked, and re-encrypted when the bookmark moves in or out of a vault
- Access: the owner and write invitees archive; anyone who can read the bookmark opens and downloads its copies. Public lists never expose them. When an admin turns archiving off, Archive… and related write controls are hidden; existing Open and Download stay available
- Limits (Admin → Quotas): archiving on or off, storage per account (unlimited by default; counted on the bookmarks you own, Trash included) and the largest copy (25 MB by default, 1 MB to 1 GB). A page or file over the cap, a bot check, a 404 or 410, or full storage fails with a reason in the drawer
- Copies stay while a bookmark is in Trash and go when it is purged, through an hourly sweep of unlisted files

---

## Vault & privacy

- Multiple named vaults, each with its own passphrase and key (Argon2id + AES-GCM)
- Vaults hold categories and loose bookmarks; move items in, out and between vaults, with a checklist when something blocks the move (including a bookmark someone else added to the folder). Only me survives the trip
- Recovery key per vault, shown once; replacing it asks for the vault passphrase. A lost passphrase and recovery key means the contents are gone
- Optional global passphrase that unlocks chosen vaults in one step, alongside each vault's own passphrase; turning it off asks for it
- Vault search skips bookmarks in Deleted items
- Vaults page: open vaults get the Library's filters, sorting, views and bulk actions, with a vault filter and lock controls; the sidebar link shows only while a vault is open
- Settings for whether vault items also appear in the Library, and whether deleted vault items go to Trash or are removed immediately
- Unlock from Settings or the command palette's Unlock vault command; locking clears decrypted data from the browser cache
- Vault content never indexed in Meilisearch
- Export includes vault bookmarks as plaintext only while the vault is unlocked for that session; locked vaults stay out of the file
- Unlocking a vault opens it for that login session only. Other devices, other sessions and API keys stay locked
- Optional auto-lock after inactivity (Settings). The server enforces it too: unlocked keys it has not used for that long are dropped. Logging out, a password change or reset, or a ban locks them. If the browser's lock request fails, the vaults stay shown as unlocked and it tries again every 30 seconds.

---

## Dashboards

- Home at `/`; library at `/library`
- Sectioned glance grid of pinned bookmarks
- Bento layout: sections sit on a 12-column grid, each with its own width (3 to 12 columns) and height (3 to 30 rows), packed upward so short sections leave no gaps above others. Bookmarks scroll inside a section that is too small for them; item grids size to their section. Collapsing a section shrinks it to its header and pulls the sections below up. On a narrow screen (under 720 px) sections stack at their natural height. The layout is kept in export and import
- **Edit layout** turns the board into the editor: drag a section by its handle to move it, pull its corner or edges to resize it, drag pinned bookmarks within their section, and open its settings (title, display, width and height, bookmarks or source) in a side drawer. Keyboard: focus a section's handle; the arrow keys move it (up and down swap it with its neighbour) and Shift with the arrow keys resizes it, announced to screen readers. Pinned bookmarks: focus one, press Space, move one place per arrow key, Space to drop, Escape to cancel
- Per-section display mode: icons only, small tiles, or medium or large bookmark cards
- Section types: **Pinned** (bookmarks you pick), **Latest**, **Most opened** (last 30 days or all time) and **Recently visited**, 4 to 24 bookmarks each (default 8)
- Latest and Most opened sources, combined: a folder with its subfolders, a tag, a website (subdomains match), favorites only, unread only. Vault bookmarks never show
- Opening a bookmark from a card, tile or icon records the open for you alone (`POST /bookmarks/{id}/open`); vault bookmarks are never recorded, and per-open history is kept 30 days
- Each section is a panel in its type's color and icon (Pinned violet, Latest blue, Most opened amber, Recently visited green), with a bookmark count, a line saying what fills it, and a menu with **See all in library** (the same folder, tag, website and flags in the library), **Edit layout** and **Collapse**
- Bookmarks show what their section adds: rank and open count in Most opened, when you opened them in Recently visited, when they were saved (and a dot for the last day) in Latest
- Collapse any section; the choice is kept in this browser. Empty sections say what will fill them, with a button to get started
- Header with the date, a greeting, and this dashboard at a glance: how many distinct bookmarks it shows, and how many of them were added this week, are unread, are favorites and were opened in the last 24 hours
- **Filter this dashboard** narrows every section by title, address, site or tag as you type; Escape clears it
- Pin a bookmark from the **+** on a pinned section without opening the layout editor
- Invitees with write access to a shared dashboard can move and resize sections and pins in the editor, and add pinned sections only
- Medium and large cards on a dashboard are built for a glance: preview, site, a two-line title (large cards add two lines of description) and a footer with the section's details (rank and opens, when opened, when saved). No tags, folders or expanding text, so cards in a row are the same height, and the whole card opens the bookmark
- Keyboard: the section under the pointer (or holding focus) gets a colored ring, its caption line turns into a "1–N open a bookmark" hint without moving anything, and its first nine bookmarks show a key number (in Most opened icons and small tiles the rank doubles as the key); press the number to open it. Press G (or ↓ with nothing focused) to jump to the first bookmark of the highlighted section without tabbing through the page; then arrow keys move within its section and between sections, with one ring in the section's color on the focused bookmark. Key hints show only with a mouse and keyboard, and can be turned off in Settings
- Sections that fill themselves draw on the owner's library, so they only exist on dashboards that aren't shared: they can't be added to a shared dashboard (or one with a pending invite), and the share drawer lists them and refuses to share until they're removed
- Multi-dashboard: create, rename, pin default, delete (fallback / recreate Home)
- Page switcher; Settings default-dashboard select
- `default_landing` `/` vs `/library` (Settings + post-login; logo honors it)

---

## Organizations and teams

- Create an organization (workspace switcher or Account menu) with a default **General** team; personal libraries stay private.
- Roles: organization owner/admin/member/guest; team maintainer/editor/viewer. Org owners/admins act as maintainers on every team.
- Email invites from **Organization settings → Members** (role + guest team); accept or decline in Notifications after a verified email; guests are team-scoped only.
- Invite links (off by default): admins set role, expiry and optional use limit; join via `/org-join` after a verified email.
- Team-space folders, bookmarks, tags and dashboards (`owner_user_id` XOR `team_id`); move personal → team (with ack); copy team → personal.
- Restricted folders: maintainer allowlist; org owners/admins always see; restrictions never widen access.
- Team archive (read-only) and soft-delete (30-day restore); organization soft-delete with name confirmation and owner email notice.
- Audit log for owners/admins in **Organization settings → Audit log**: filter by person, action and date; **Download CSV** (same filters). Append-only; 1-year retention prune.
- **Verified domains:** owners/admins add a domain, publish DNS TXT `_linkdean-challenge.<domain>` = `linkdean-domain-verification=<token>`, then Verify. Per-domain join mode: auto-join matching verified emails as members, or require admin approval of join requests. Consumer and disposable mail domains are refused.
- **Organization SSO (OIDC) + SCIM:** owners/admins configure one OpenID Connect IdP and a SCIM bearer token in **Organization settings → SSO**; enable SSO only after a verified domain. Members use **Continue with company SSO** on the login page. IdPs can provision/deprovision members at `/api/scim/v2/Users` with an `ldscim_` token. Client secrets encrypted at rest. SAML deferred. Design: [organization SSO and SCIM](../design/organization-sso-scim.md).
- Sidebar **workspace switcher** (always shown: Personal, organizations, Create organization) and organization settings (members + email invites, teams, invite links, domains, SSO, audit, danger zone).

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
- A dashboard in its owner's Trash is hidden from the people it is shared with until the owner restores it
- Nested shares: each folder takes its nearest grant, so a read share inside a write share is read from there down
- Tags a write invitee sets on a shared bookmark go into the owner's tags
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
- Notification emails (needs mail): **Settings → Notifications** sets each kind to **Off**, **Each one** or **Daily digest**. Share invites and moderation changes are on by default and always go out at once; access changes, revoked shares, shared edits, likes, comments, ratings and new followers are off. The same block and mute rules apply, and each email links back to that settings tab. With mail off the tab says email needs outbound mail and the choices are disabled
- Daily digest: kinds set to **Daily digest** arrive together in one email a day, after 8:00 in your profile's time zone (UTC when none is set), grouped by kind with up to 50 items and a count of the rest. Switching a kind back to **Each one** sends what was waiting; **Off** drops it
- One-click unsubscribe: every notification email links an unsubscribe for its kind (the digest's turns off every digest kind) and carries `List-Unsubscribe` and `List-Unsubscribe-Post` headers, so mail apps show their own Unsubscribe button. The link works signed out and opens a page that asks first, so link scanners change nothing. Links are signed with `SESSION_SECRET`; changing it voids them
- Likes, comments, and 1–5 ratings on **shared** bookmarks, lists, and dashboards
- Follow / unfollow users (⋮ on shared bookmarks; lists on Profile; notifies the followee). Follower counts show only to people who can see the profile
- Block / unblock users (⋮ on shared bookmarks; list on Profile): removes follows and shares both ways without telling them, and hides each other's social activity and notifications. An invite from someone you blocked reads as sent to them but never reaches you
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
- Optional OIDC sign-in with any standard issuer (discovery, PKCE, nonce and ID token checks), with its button named by `OIDC_NAME` (for example "Authentik")
- Sign-in presets next to it: **Google**, **Microsoft** (Entra ID, one tenant) and **GitHub** (OAuth, verified primary email from the GitHub API). Each configured provider gets its own **Sign in with …** button and its own **Connect …** button under **Profile → Edit profile → Single sign-on**. An account still holds one provider identity
- Passkeys: **Profile → Edit profile → Passkeys** adds one (current password, or a fresh single sign-on for accounts without one), lists them with when they were added and last used, and renames or removes them; the account gets a "A passkey was added" email. **Sign in with a passkey** on the login page needs no email: the browser offers the passkeys it holds for the site. Up to 20 per account; failed passkey sign-ins count against the address's sign-in limit
- Single sign-on linking: an OIDC sign-in whose address belongs to a password account asks for that account's password once, then connects; the account gets a "Single sign-on was connected" email. **Profile → Edit profile → Single sign-on** shows the connected provider, with **Connect** (any provider address) and **Disconnect** (needs the password; not offered to SSO-only accounts)
- First registered user becomes admin (bootstrap)
- Admin page in sections, like Config options: General, Users, Reports, Bans, Invites, Quotas, Email, Screenshots and Search index (`/admin?tab=…`), down the side on wide screens and as a scrolling row on phones
- Open registration toggle (Admin); public beta keeps registration open and relies on the protections below
- Admin registration invites: one-time `ldi_…` codes / copyable `/register?invite=` links (optional email bind); required when registration closed
- Optional Cloudflare Turnstile on password registration (`TURNSTILE_SITE_KEY` / `TURNSTILE_SECRET_KEY`); off when unset; OIDC uncapped by Turnstile
- Optional disposable-email domain block at register and email-change (`BLOCK_DISPOSABLE_EMAILS`)
- Per-user quotas (Admin platform settings; null = unlimited): max bookmarks, stored image bytes, enrich actions per UTC day, import size, upload image size (clamped to hard caps), archive storage. On the managed service (`MANAGED_SERVICE`), Free/Pro personal plans merge with those caps (tighter wins), including vault count on Free; Settings shows the plan name and usage. With Stripe keys set, Settings offers **Upgrade to Pro** / **Manage billing** (Checkout + Customer Portal); webhooks keep `users.plan` in sync. Admins can still set Free/Pro for comps. Self-host stays admin quotas only. Tier table: [plans and pricing](../design/plans-and-pricing.md)
- Optional `BOOTSTRAP_TOKEN`: the first account (the admin) must enter it on the setup page. Racing sign-ups cannot make two first admins or redeem one invite twice
- Profile: display name, bio, avatar, password change, email change. A new password signs out every other session, and can revoke all API keys too
- Password reset: with mail on, **Forgot password?** emails a single-use link that works for 30 minutes; admins can make a 24-hour link to hand over, the only way back in when mail is off. A reset signs out every session, can revoke all API keys, and leaves vaults locked
- Email verification (needs mail): a password registration emails a 24-hour, single-use link, and unverified accounts see a banner with **Resend email**. Opening the link verifies the address without signing in. OIDC accounts are verified when the provider reports the address as verified. Accounts that existed before verification shipped count as verified
- Email change: **Profile → Edit profile → Change email** with the current password sends a link to the new address and a notice to the old one; the address changes only when the link is opened. Not for SSO-only accounts; with mail off it says to ask an admin
- Admin verification controls: an **Unverified** badge in the accounts list, **Mark verified**, **Remove verification**, and **Change email** applied at once
- **Require verified email** (Admin, needs mail): unverified accounts can only verify or sign out until they do. Admins are never held. Host env `REQUIRE_VERIFIED_EMAIL=true` forces the same gate and locks the admin toggle
- Admin accounts list: promote / demote admin (last admin protected), per-user API-keys gate
- Session auth; multi-device via server account
- Delete account (Settings → General): confirm email, plus the password when the account has one or a fresh single sign-on otherwise (**Sign in again** sends an SSO-only account to its provider with `prompt=login`, and the step-up lasts 10 minutes in that session only); 7-day grace with cancel, then hard purge of owned data including shared folders. The last admin cannot schedule, and the purge checks again: when the instance would be left with no active admin, that account stays scheduled until another admin exists
- Admin bans with a reason, for N days, until a date or permanently; a banned user sees why on `/banned`, and API keys stop working while the ban lasts
- Legal pages: the operator's own Terms of Service and Privacy Policy (`TERMS_URL`, `PRIVACY_URL`), linked from sign-in, sign-up and the accept prompt. `/terms`, `/privacy` and `/acceptable-use` (to `#acceptable-use`) on the app redirect to them. Unset, the app shows no legal links. linkdean.app's own are at [linkdean.app/terms](https://linkdean.app/terms) and [linkdean.app/privacy](https://linkdean.app/privacy)
- Terms at sign-up, on a server with `TERMS_VERSION`: password sign-up and first-admin setup need a terms checkbox and the API refuses a missing or stale `terms_version`; SSO sign-in needs the checkbox too (`/auth/oidc/start?terms_version=`), and a new SSO account records it. The version and time are stored on the user (`users.terms_version`, `terms_accepted_at`)
- Updated terms: when `TERMS_VERSION` changes, every account on an older version (or none) sees a **Our terms changed** dialog in the app, with links to the Terms and the Privacy Policy, **Accept and continue** or **Sign out**. Accepting records the new version (`POST /api/users/me/terms`). API keys are not blocked
- Beta banner on every page of a server with `TERMS_URL` (no uptime promise, export regularly), dismissible for the browser session
- Report abuse (signed-in users): a **Report** control on public lists (`/l/…`, sign-in link when signed out), other people's profiles and other people's comments, with a reason (spam, harassment, illegal, impersonation, other) and optional details. Reporting your own content is refused; a second report of the same target while yours is open returns the first; 10 reports per 15 minutes per user
- Admin report queue (Admin → Abuse reports): open reports with reporter, reason and details. **Dismiss** closes it; **Hide content** revokes the public link, hides the profile or deletes the comment (recorded in the user's moderation history); **Warn** records a note and emails the user when mail is on; **Ban** bans the user with the note (or the report reason) and still resolves the report if they are already banned
- Abuse contact: admins set an email in the same panel (`platform_settings.abuse_contact_email`); the report form shows it. Terms and Privacy tell people to use Report or that address. Marketing site footer also lists `contact@linkdean.app`
- Banned users list with bulk unban and bulk change of reason or duration
- A banned user's public lists stop loading for anyone while the ban lasts, and come back when it ends or is lifted
- Moderation of a banned user's exposed content: turn off public links, set items private, hide the profile, revoke shares, delete comments, move items to the owner's Trash; private and vault content stay out of reach, and every action is recorded

---

## API keys

- Users create and revoke keys scoped to their own data (Settings → API keys). At create time they pick **permissions** (`library` / `archive` / `import_export` / `social` / `account`, each read and/or write), with presets (read-only library, save bookmarks, full access) or custom checkboxes. REST and MCP refuse calls the key lacks. Existing keys and omitted scopes keep full access; the browser extension still mints a full-access key. Sessions are never limited by scopes.
- Bearer `ldk_…` accepted alongside session
- A key cannot create more keys or use admin routes, even an admin's, so revoking a leaked key ends its access
- Secret hashed at rest; plaintext shown once at creation
- Platform-wide and per-user enable/disable (Admin)
- **API reference**: an OpenAPI 3.1 document generated from the routes, served at `GET /api/openapi.json` and published as the [API reference](https://docs.linkdean.app/docs/api-reference) on the docs site. It lists every endpoint with its auth (API key, sign-in session only, or public), parameters, request and response bodies
- **One error shape**: every error answers `{ "code", "message", "details"?, "error" }`. `code` is stable (`not_found`, `session_expired`, `quota_exceeded`, …), `message` is for people, `details` carries a ban, a quota or what blocked a vault move, and `error` repeats `message` for older clients. Malformed JSON, unknown methods and oversized bodies answer in the same shape
- **Caching headers**: successful JSON reads carry a strong `ETag` and `Cache-Control: private, no-cache`, and a matching `If-None-Match` gets `304 Not Modified`. Writes, errors and downloads are `no-store`
- **Browser extension sign-in** without handling a key: the extension opens `/connect/extension` in a window of its own, the person signs in as usual and chooses **Allow**, the page hands a single-use code to the extension's script on that page, and the extension trades it (two minutes, PKCE S256, bound to the extension's `chrome-extension://` or `moz-extension://` address) at `POST /api/auth/extension/token` for a key named "Browser extension · Chrome" or "· Firefox". Approving needs a login session, never a key. Signing out in the extension revokes its key. Needs API keys enabled for the account
- **Saved-page icon** (opt-in, in the extension's popup): the toolbar icon is in color on pages already in your library and grey elsewhere. The first popup opened after sign-in offers it once (**Allow** / **Not now**); the switch below the save form changes it later. Turning it on asks for the `tabs` permission; turning it off gives it back. The extension downloads short hashes of your saved addresses (`GET /api/bookmarks/url-hashes`: the first 16 hex digits of SHA-256 over each exact URL, outside vaults and the trash), refreshes them every 5 minutes, and checks each tab on the device, so pages you only visit are never sent

---

## Import / export

- Import: Netscape HTML, Linkdean JSON, Pocket CSV, Raindrop CSV
- Imported bookmarks get metadata and previews in the background
- Export: Netscape HTML, Linkdean JSON (full account export of owned folders, bookmarks and dashboards; vault items included while unlocked). Optionally **Include archives** to download a zip with the export file plus every archived copy you can read (locked vault copies are skipped)
- **Agent Skills / MCP** (experimental, off unless the server sets `EXPERIMENTAL_AGENT_SKILLS=true`; [guide](../guides/agent-skills.md)) for coding and research agents (Claude Code, Codex, Cursor and others):
    - **MCP** (live, preferred): connect a client to `/api/mcp` (Streamable HTTP) with `Authorization: Bearer ldk_…`. Tools search and read bookmarks, list folders and tags, return archive metadata and reader Markdown (`get_archive_text`), and — when the key has `library:write` — create, update or trash bookmarks. Vault items and raw archive file bytes stay out. The **MCP** card under **Config options → Import / Export** shows the URL and key steps
    - **Agent Skill zip** (offline): a folder (with its subfolders), a tag, or a saved library search becomes an [Agent Skill](https://agentskills.io), a `SKILL.md` whose description tells the agent when to use it, with the links grouped by subfolder, their descriptions, tags and your notes. Saved-search skills use live matches (vault always excluded). Download from **Config options → Import / Export → Agent Skill**, a folder's **⋯ → Agent Skill (.zip)**, or a saved search's download control, or fetch with an API key from `GET /api/agent-skill?folder=<id>`, `?tag=<name>`, or `?search=<id>` (`&format=md` for the `SKILL.md` alone) so a script can keep it current
    - Over 100 links in a zip skill move to `references/LINKS.md`, which the agent reads only when needed; a skill holds up to 2,000 bookmarks and says how many it left out
    - Never includes vault folders or bookmarks, deleted bookmarks, someone else's "Only me" bookmarks or their notes. Shared folders work with read access; a tag covers your own bookmarks

- Linkdean JSON (version 2) keeps every folder of a bookmark, its favorite, unread, pinned and private flags, its created and updated times, and your dashboards. Importing adds the dashboards beside your own, with each section's folder pointing at the imported folder
- Netscape HTML lists a bookmark under each folder it is in, and reads `<DD>` descriptions on import
- Duplicates on import: before anything is written, Settings → Import checks the file against your bookmarks outside vaults by exact URL. When some are already in the library, a review lists each one beside the existing bookmark with **Skip** (the default), **Replace** (update the existing bookmark in place with the file's title, description, tags and folders, only those the file has, keeping its shares, pins and activity), **Merge** (keep the existing title and description, fill only empty ones from the file, and add the file's tags and folders to the bookmark's own) or **Import anyway**, per row or for all. The API takes the same choices: `POST /api/import/preview`, then `duplicates` on `POST /api/import`; without them every bookmark is created
- Folders in an imported file whose full path already exists (exact names, outside vaults) are reused instead of created again, so importing the same file twice does not duplicate them
- One import at a time per user, and at most 10 per 15 minutes; previews have their own 30 per 15 minutes. A file may hold up to 10,000 folders (Raindrop CSV included), 100 dashboards and 100 sections per dashboard

---

## Ops & self-host

- Docker Compose stack (API, Postgres, Meilisearch, optional Chrome)
- Release images: a `v*` tag runs CI, publishes `ghcr.io/arsamsarabi/linkdean:<version>` and creates a GitHub release; [quick-start](../self-hosting/quick-start.md) has the Compose file and optional Chrome setup, and [environment](../self-hosting/environment.md) is the full variable reference (also on [docs.linkdean.app](https://docs.linkdean.app/docs/self-hosting/quick-start))
- Public docs site: [docs.linkdean.app](https://docs.linkdean.app), with content from this `docs/` tree
- Public marketing site: [linkdean.app](https://linkdean.app)
- Public app: [app.linkdean.app](https://app.linkdean.app), the hosted public beta
- Version at the foot of the sidebar (and in `GET /api/auth/config`). Admins see a badge when a newer GitHub release is out: blue for a patch, teal for new features, orange for a major release and red when any release in between has a `### Breaking` heading in its notes (the release workflow copies it from that version's CHANGELOG section). The badge links to the release notes. Only the server asks GitHub, at most every 6 hours (`GET /api/admin/update`); `UPDATE_CHECK=false` turns it off
- Healthchecks on db / Meili / api; `GET /api/health` (reports the running version) and `GET /api/ready`
- Security headers on every response (`Content-Security-Policy`, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`, `X-Frame-Options`; optional `Strict-Transport-Security` via `HSTS_MAX_AGE`). Image routes keep their own sandbox CSP. `/.well-known/security.txt` and `/security.txt` point to the project security contact
- Supply chain: CI runs `cargo audit` and `pnpm audit --prod` on every PR; Dependabot opens updates for Cargo, npm and GitHub Actions
- Chrome on an internal network with no route out: it reaches the web only through the API's egress proxy, which resolves each host itself and refuses private addresses
- Prometheus scrape: `GET /api/metrics` (HTTP counters / latency histograms by route template + process metrics), behind `METRICS_TOKEN`
- Structured logs: `LOG_FORMAT=text|json` on stdout (default text); optional `LOG_FILE` appends JSONL; every response has `X-Request-Id` correlated into request log spans. Request spans record the path, never the query string
- Optional OpenTelemetry tracing: with `OTEL_EXPORTER_OTLP_ENDPOINT` set, request spans go to an OTLP/HTTP collector (Jaeger, Tempo, Honeycomb, Grafana Cloud, …) carrying the same `request_id` as the logs
- Outbound email over SMTP (optional): an outbox with retries, Admin → Email with status, delivery counts and a test send; Mailpit catches dev email and `/api/dev/emails` previews templates
- Avatars and preview volumes; backup recipe in [../self-hosting/operations.md](../self-hosting/operations.md)
- Sidebar navigation (Dashboard, Library, Vaults, quick views, the folder tree with Unsorted, Library health, Deleted items, settings); on phones the burger slides it in. On tablets and wider, the header's sidebar button (or ⌘B / Ctrl+B) collapses it to a narrow icon rail with tooltips (main pages, recent folders, Deleted items, settings), remembered per browser; expand again for labels, quick views and the folder tree
- Shared page (sidebar → Shared, with a count of invites waiting): accept or decline invites, open folders and dashboards others share with you, and see what you share and with whom; `/?board=<id>` opens a given dashboard
- Config options: sections (General, Notifications, Fast capture, Import / Export, Vault, API keys) listed down the side on wide screens and as a scrolling row on phones; `?tab=` opens one
- Sidebar Folders section collapses: collapsed it shows Unsorted, then the 3 folders with the latest activity (a bookmark added or changed in them, or the folder itself made or renamed) and the open folder; expanded it shows the whole tree with a filter field (matches at any depth, with their path). The choice is remembered per browser
- Phone tab bar: Home, Library, Add (into the open folder), Search (command palette) and Me; the burger keeps the full sidebar
- An open folder titles the library page, with a breadcrumb back to the whole library (Unsorted included); an empty folder offers Add bookmark here
- Form drawers (bookmark, bulk edit, new and edit category, report, edit profile) ask "Discard changes?" with **Keep editing** and **Discard** before Escape, a click outside, the close button or Cancel throws away unsaved changes. An unchanged form closes at once (the details quick save fills in from the page do not count as a change); saving never asks
- On systems whose scrollbars take up space, the page, drawers and modals scroll under a thin overlay scrollbar instead, so there is no empty gutter and content does not shift sideways when a scrollbar appears or a panel opens
- Header search field opens the command palette (which can also add a bookmark); the library page's Add bookmark button, the phone tab bar's Add and paste-to-add open the new-bookmark form
- **Show keyboard hints** (Settings → General → Keyboard, on by default): turn it off for a cleaner look, hiding the key numbers on dashboard bookmarks, key hint lines (dashboard footer and section hint, filing queue, command palette footer) and shortcut keys such as ⌘K and /. Shortcuts keep working
- Editing a shared bookmark: the side panel has Details and Discussion tabs, so likes, rating and comments sit next to the edit form (the card menu's Likes & comments still opens them on their own)
- Paste a web address anywhere outside a text field to open the new-bookmark form with it filled in (in the open folder when on the library)
- PWA install: web manifest + service worker (standalone / “Add to Home Screen”)
- Hardened before release: race-safe sign-up, invites and admin demotion; vault moves lock their rows; the browser session lives only in the `HttpOnly` cookie, with a client header guarding writes; Chrome runs sandboxed, non-root and off the data network; a `.dockerignore` keeps local files out of the image

---

- Optional page-view statistics with GoatCounter (`GOATCOUNTER_URL`, off when unset): one beacon per route with the route's name only, never ids, search terms or titles; no cookies or browser storage; Do Not Track and Global Privacy Control are honored; the CSP allows beacons to that origin only. linkdean.app counts its app, marketing site and docs on stats.linkdean.app
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
| `/terms`, `/privacy`, `/acceptable-use` | Redirect to `TERMS_URL` and `PRIVACY_URL` (`/acceptable-use` to the terms' `#acceptable-use`); the not-found page when unset |
| `/trash` | Soft-deleted items |
| `/profile` | Account profile |
| Command palette | Quick add / search / navigate |
| [linkdean.app](https://linkdean.app) | Marketing (static) |
| [docs.linkdean.app](https://docs.linkdean.app) | Documentation (static) |
| [app.linkdean.app](https://app.linkdean.app) | Hosted app (public beta) |
