# Changelog

## [Unreleased]

### Added

- Redesigned dashboard: each section is a panel in its own color with an icon, a count, a **See all in library** link and a collapse toggle; bookmarks show their rank and opens, when you opened them, or when they were saved. The header shows the date, a greeting and your library at a glance, with a filter for the whole dashboard. Pin to a section from its **+**, and open bookmarks with `1` to `9`.
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

- The library list filters, sorts and pages in the database, so each page costs the same however large the library is.
- Privacy Policy (version 2026-11-11): describes page-view statistics, and what the extension's saved-page icon reads.
- **Import from URL** on the bookmark form now replaces the title and description with the page's, and adds its tags, instead of filling only empty fields.
- Restructured `docs/` into purpose folders (`self-hosting/`, `product/`, `guides/`, `ops/`, `design/`); install split into quick start, environment, and operations.

### Fixed

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
