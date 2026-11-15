# Changelog

## [Unreleased]

### Added

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
- Create a category from the bookmark form: type a name that matches no category, pick **Create "name"**, and choose where it goes.
- Import checks for bookmarks already in your library and lets you skip, replace, merge or import each one again; folders that already exist are reused.
- Explorer folder tiles show how many bookmarks each folder holds again.
- **Config options → Fast capture** links to the browser extension on the Chrome Web Store.
- Admins can retry failed screenshots from **Admin → Screenshots**, once per cooldown, and set how long failed screenshots wait before the automatic retry, which now runs every 6 hours instead of only at start.

### Changed

- Rewritten Terms of Service and Privacy Policy. The Terms add operator details, a content license, intellectual property, a liability cap, governing law and general clauses, and acceptable use moves to section 6. The Privacy Policy adds the data controller, legal bases, international transfers, retention, your rights and children. Operator names and contact addresses are still placeholders.
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
