# Changelog

## [Unreleased]

### Breaking

- **Archives volume.** Self-hosters who keep a Compose file: add a named `archives` volume mounted at `/app/data/archives` on the API (see [quick-start](self-hosting/quick-start.md) and [environment](self-hosting/environment.md) for `ARCHIVE_DIR`). Without it, archived copies are lost when the container is replaced. Include that volume in backups. HTML and PDF page captures also need Chrome (`BROWSER_WS_URL`); original-file archives do not.

### Added

- **Search inside archived pages**: library search matches words from the body of non-vault HTML archives after the copy finishes. Vault bookmarks never contribute page text. Existing HTML copies gain body search when you archive again.
- **Version in the sidebar**: the app's version (such as `v0.0.12`) sits at the foot of the sidebar. Admins also see a badge when a newer release is out, colored by how risky it is to update (patch, new features, major or breaking), which opens the release notes. The server checks GitHub at most every 6 hours; self-hosters turn it off with `UPDATE_CHECK=false`. A release counts as breaking when its section in this changelog has a `### Breaking` heading: the release workflow copies that heading into the GitHub release notes.
- **Playwright @smoke on PR CI**: Chromium journeys in `apps/web/e2e` run against a throwaway Postgres, Meilisearch and API on every pull request (`Web e2e (@smoke)` job).
- **Archive**: keep a copy of a bookmark as a web page (single-file HTML) or PDF, or the file itself when the address serves one, from its menu, the bookmark drawer or the Library's select mode, or for every new bookmark (**Settings → Archive**). Open the copy inside Linkdean at `/archive/{id}`, download it, archive again or delete it. Broken-link badges and Quick peek offer the copy. Vault copies are encrypted with the vault key. Admins turn archiving off or cap storage per account and the largest copy in **Admin → Quotas**. Self-hosters: add the `archives` volume at `/app/data/archives` (see the Compose files); pages need Chrome (`BROWSER_WS_URL`).
- **Firefox add-on**: the browser extension is on [Firefox Add-ons](https://addons.mozilla.org/firefox/addon/linkdean/), and **Config options → Fast capture** links to it beside the Chrome Web Store.
- **Passkeys**: add one under **Profile → Edit profile → Passkeys**, then **Sign in with a passkey** from the login page, with no email or password to type.
- **Sign in with Google, Microsoft or GitHub**: self-hosters can turn on each with its own client ID and secret, next to any OpenID Connect provider such as Authentik (now named on its button with `OIDC_NAME`).
- **API reference**: every endpoint documented on the docs site, from an OpenAPI document the API also serves at `/api/openapi.json`.
- **OpenTelemetry tracing** for self-hosters: set `OTEL_EXPORTER_OTLP_ENDPOINT` to send request traces to your collector.
- **Quick peek**: open a bookmark inside Linkdean from a card, row or dashboard tile. Maximize, minimize, detach, open in a new tab, or close with Esc; size is remembered. Vault bookmarks stay out. Sites that block embedding fall back to preview, title, description and **Open in new tab**. Peeking counts as an open.
- **Icon-only sidebar rail**: on tablets and wider, ⌘B (or the header button) collapses the sidebar to a narrow strip of icons with tooltips instead of hiding it. Phones keep the burger menu and bottom tabs.
- **Updated terms prompt**: when the Terms of Service change, the app asks you to accept the new version, or sign out, before you carry on.
- **Sign in again before deleting an SSO account**: an account without a password now confirms with a fresh single sign-on before it can schedule deletion, instead of the email alone.
- **Daily notification digests and one-click unsubscribe**: in **Settings → Notifications** each kind is now **Off**, **Each one** or **Daily digest**, and digest kinds arrive in one email a day after 8:00 in your time zone. Every notification email has an unsubscribe link for its kind and `List-Unsubscribe` headers, so mail apps offer one-click unsubscribe; it works signed out and asks before turning anything off.
- **Agent Skills** (experimental, off by default; set `EXPERIMENTAL_AGENT_SKILLS=true`): turn a folder or a tag into an [Agent Skill](https://agentskills.io) for Claude Code, Codex, Cursor and other agents, so they use your curated links instead of searching the web. Download it from **Config options → Import / Export** or a folder's menu, or fetch it with an API key (`GET /api/agent-skill`). Vault bookmarks are never included.
- **Show keyboard hints** setting (Settings → General → Keyboard): turn it off to hide key numbers, key hint lines and shortcut keys for a cleaner look. Shortcuts keep working.
- **Bento dashboard**: sections sit on a grid, each with its own width and height, and pack upward to fill the gaps. In **Edit layout**, drag sections by their handle, resize them from the corner or edges (or with Shift and the arrow keys), drag pinned bookmarks to reorder them, and change a section's settings in a side drawer. Existing layouts carry over. Phones stack sections in one column.
- Redesigned dashboard: each section is a panel in its own color with an icon, a count, a **See all in library** link and a collapse toggle; bookmarks show their rank and opens, when you opened them, or when they were saved. The header shows the date, a greeting and counts for the bookmarks on that dashboard, with a filter for the whole dashboard. Pin to a section from its **+**, and open bookmarks with `1` to `9`.
- **Dashboard upgrades**: each section picks how its bookmarks look (icons, small tiles, or medium or large cards), and new sections fill themselves with your latest bookmarks, the ones you open most (last 30 days or all time) or the ones you opened last, filtered by folder, tag, website, favorites or unread. They stay on dashboards you haven't shared.
- Form drawers ask before discarding unsaved changes: closing an edited bookmark, category, bulk edit, report or profile form by Escape, a click outside or Cancel offers **Keep editing** or **Discard**.
- Optional page-view statistics: set `GOATCOUNTER_URL` to count which pages of the app are used, by route name only, without cookies, and never from browsers that send Do Not Track or Global Privacy Control.
- Browser extension: an opt-in toolbar icon that is in color on pages already in your library and grey elsewhere, checked in the browser against hashes of your saved addresses.
- Browser extension: the first popup opened after signing in offers the saved-page icon once, with **Allow** and **Not now**. The `tabs` permission stays optional, so installing shows no extra warning.
- **File one by one**: a filing queue that walks through Unsorted bookmarks, suggesting folders and tags you already use on the same site, with keyboard shortcuts and swipes. The sidebar shows how many bookmarks are in Unsorted.
- Create a category from the bookmark form: type a name that matches no category and pick **Create "name"** to create it at the top level and select it.
- Import checks for bookmarks already in your library and lets you skip, replace, merge or import each one again; folders that already exist are reused.
- Explorer folder tiles show how many bookmarks each folder holds again.
- **Config options → Fast capture** links to the browser extension on the Chrome Web Store.
- Admins can retry failed screenshots from **Admin → Screenshots**, once per cooldown, and set how long failed screenshots wait before the automatic retry, which now runs every 6 hours instead of only at start.

### Changed

- **Your own legal pages**: the app no longer ships linkdean.app's Terms of Service and Privacy Policy. Set `TERMS_VERSION`, `TERMS_URL` and `PRIVACY_URL` to ask for acceptance at sign-up and link your own pages. Without them, sign-up asks for no terms and the app shows no legal links or beta notice. `/terms`, `/privacy` and `/acceptable-use` redirect to those pages.
- Rewritten Terms of Service and Privacy Policy. The Terms add operator details, a content license, intellectual property, a liability cap, governing law and general clauses, and acceptable use moves to section 6. The Privacy Policy adds the data controller, legal bases, international transfers, retention, your rights and children. The operator is Arsam Sarabi (sole trader, trading as Linkdean), with contact@, privacy@ and abuse@linkdean.app, under the law of England and Wales. The Privacy Policy now matches the deployment: Cloudflare in front of every host, Turnstile on sign-up, self-hosted page view counts, iCloud Mail, passkeys and concrete retention periods.
- The Acceptable Use Policy is now section 5 of the Terms of Service, so there are two legal documents to read and accept instead of three. `/acceptable-use` redirects there. The terms version is now 2026-11-15, so every account is asked to accept the updated Terms once.
- **API errors** all answer `{ code, message, details?, error }`; ban, quota and vault-move data moved into `details`. `error` still holds the message.
- **API reads** send `ETag` and `Cache-Control`, so unchanged data comes back as `304`.
- Account deletion checks for the last admin again when the purge runs. If every other admin left in the meantime, the account stays scheduled until another admin exists.
- Dashboard cards are redesigned: preview, site, title, and a footer with rank and opens, when you opened it or when it was saved. No tags, folders or "More" text, so cards in a row line up. Pointing at a section no longer pushes its header down: the key hint now replaces the caption line in place.
- Bookmark cards: folders now sit right under the title, shown as breadcrumbs, and tags hide behind a **N tags** toggle so cards stay short. List rows open tags in a popover.
- The library list filters, sorts and pages in the database, so each page costs the same however large the library is.
- Privacy Policy (version 2026-11-11): describes page-view statistics, and what the extension's saved-page icon reads.
- **Import from URL** on the bookmark form now replaces the title and description with the page's, and adds its tags, instead of filling only empty fields.
- Restructured `docs/` into purpose folders (`self-hosting/`, `product/`, `guides/`, `ops/`, `design/`); install split into quick start, environment, and operations.

### Fixed

- Upgraded `lettre` to 0.11.24 for the SMTP security advisories in RUSTSEC-2026-0560…0565.
- Library and dashboards:
  - **Create "name"** on the bookmark form creates the category at once and shows **Category created**, instead of opening a second panel people often missed before saving to Unsorted.
  - Search results show the bookmark's real folders (and catch Meili up when tags or folders changed), instead of an **Unsorted** chip after a reload.
  - The bookmark URL field checks on blur and blocks Add until the address is a real `http(s)` URL, so a stray paste like `not a url` no longer saves as a dead link.
  - **Move to vault** with no vault yet offers **Set up vault**, instead of only **Out of vault** and a note about locked vaults.
  - Bookmarks with no saved site icon show a globe, instead of asking Google for one: the request was blocked anyway, logged an error per tile, and would have sent each bookmarked site to Google.
  - When a bulk tag, folder or delete fails for some bookmarks, the notice gives the reason and those bookmarks stay selected to retry.
  - Bulk **Move to vault** leaves out bookmarks shared with you, which can't go in a vault, instead of failing the whole move.
  - A tag can't contain a comma any more (it splits into two tags, as the tag inputs already did), so every tag can be filtered.
  - Picking the same image again after cancelling a preview crop opens the crop again.
  - After switching accounts in the same browser, the sidebar no longer shows the previous account's avatar.
  - A dashboard edit can no longer be saved onto a different dashboard after losing access to the one being edited.
  - Importing a Linkdean export keeps a dashboard section's folder (such as "Latest in Work"), instead of drawing on the whole library.
- Sharing and privacy:
  - Sharing with someone who blocked you looks like any other invite, so you can't tell. It never reaches them.
  - A dashboard in its owner's Trash can no longer be opened or edited by the people it is shared with. Restoring it brings their access back.
  - Tags that a write invitee sets on a shared bookmark now go into the owner's tags, instead of disappearing from the owner's tag list and filters.
  - With a read share inside a write share, the inner folder and everything below it show read access, as the server enforces, instead of offering edits that then fail.
  - A stale pin of a bookmark on someone else's dashboard no longer blocks moving it into a vault, and the blocker no longer shows that dashboard's title. The move removes such pins.
  - Follower and following counts show only to people who can see the profile, and following someone across a block answers like an unknown user.
- Vaults:
  - When the idle auto-lock request fails, vaults stay shown as unlocked, as they still are on the server, and the lock is retried every 30 seconds.
  - Vault bookmarks are tied to their row and vault, so a copied ciphertext no longer opens on another bookmark. Existing ones gain this the next time they are saved.
  - The global passphrase's public key is checked whenever the master key opens, and put back if it was changed.
  - Unlocked vault keys are wiped from server memory when dropped and never appear in debug logs.
  - Moving a folder into a vault names a bookmark someone else added to it (while it was shared) as something to move or delete first, instead of failing with "not found".
  - A bookmark set to Only me keeps that setting after moving into a vault and back out.
  - Moving a folder out of a vault no longer brings its deleted bookmarks back into search.
- Records of individual bookmark opens are now deleted after 30 days for everyone. Before, they were only pruned when the same person opened another bookmark.
- The collapsed sidebar rail centers its icons in square buttons, marks the current page with a highlight and an edge bar, and separates recent folders from the main pages.
- Docs: the **Intro** and **Getting started** pages show their screenshots again, instead of a stray `import Shot …` line.
- Dashboard: key numbers are no longer clipped at the top of a section, small tiles have more room between them, the focused bookmark has one ring instead of two. Press **G** to jump straight into the sections.
- Side drawers no longer spill past their right edge when a field holds a long item, such as a bookmark with a long title in the section settings, and are a little wider (520 px instead of 440 px) on tablets and desktops.
- Browser extension: the first **Sign in** click now opens the Linkdean window. Chrome closed the popup while it asked for the site permission, so it took a second click.
- The **How to set it up** link under **Config options → Fast capture** opened a missing docs page; it now opens the iPhone Shortcut guide.
- Paging the library no longer skips bookmarks that share the same updated or created time.
- The API shuts down cleanly on `docker stop`: it finishes open requests for up to 8 seconds and exits, instead of being killed after Docker's 10-second timeout.
- Docs pages load their own content directly, so links, search engines and link previews see the right page; unknown docs addresses return a 404 page.
- Docs edits trigger a docs-site deploy again after the restructure; fixed links the move broke.

## [0.0.2] - 2026-11-06

### Added

- Public documentation site at [docs.linkdean.app](https://docs.linkdean.app) (`apps/docs`, synced from `docs/`).
- Public marketing site at [linkdean.app](https://linkdean.app) (`apps/site`).
- Homelab public-beta deploy package (`deploy/public-beta/`) for the Nova LXD guest + Cloudflare Tunnel.
- Contributing guide, security policy, code of conduct and pull request template.
- Path-filtered release deploys on a Nova self-hosted runner.
- PolyForm Noncommercial 1.0.0 project license (prior MIT-tagged trees remain MIT).

### Fixed

- Finishing onboarding no longer sends you straight back into it (vault setup and import again).

## [0.0.1] - 2026-11-04

### Added

- Initial release: the Docker image `ghcr.io/arsamsarabi/linkdean:0.0.1` and the install guide (now `docs/self-hosting/quick-start.md`).
