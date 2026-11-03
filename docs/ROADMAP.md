# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`. None of them block a public release today.

---

## Now: before the first public release

1. **Outbound email** (M–L). The mail foundation (SMTP, MJML templates, outbox, Admin panel) and password reset ship. Still to build on it:
   - Verify email addresses at registration and on email change, before the address can receive share invites. See the known-issue note on `create_share` in `crates/db/src/repo/shares.rs`.
   - Until an address is verified, invites to it wait as pending invites for the address rather than going to the unverified account.
   - An approved link flow, so a local account with someone's email cannot block their first OIDC sign-in.
   - Email delivery for invites and notifications.

2. **Page the library list** (L). `GET /bookmarks` returns every bookmark, and the web app filters and sorts the whole set in the browser (`useLibraryList` → `filterAndSortLibraryItems`). Large libraries pay for that on every load, and an import of thousands of bookmarks makes the app slow from day one.
   - Use infinite scrolling, not numbered pages. The library is browsed and filtered rather than jumped through by page, and filters change the result set often.
   - Keyset pagination in the API (`cursor` + `limit`, ordered by the active sort key and `id`), so new or edited bookmarks do not shift or repeat rows the way `OFFSET` would.
   - Move folder, tag, flag, "since" and sort filters into the query, and return the total match count for headers and "select all".
   - Explorer loads one folder's contents at a time, rather than slicing the full list.
   - Bulk actions on "select all" act on the server-side match (filters sent to the API), not only the rows loaded so far. Keep H21's rule: select all reaches only what the open folder or search shows.
   - Vault search stays a Postgres scan of unlocked rows. Page it the same way.
   - Batch the access checks for bookmarks shared with the viewer, which still run one at a time.

3. **Final UI/UX and mobile pass** (L). Before the public release, review the whole UI with a strong UI/UX skill and reimagine it where needed. Make every page responsive and usable on phones, not only the nav.
   - Mascot art (`brand/graphics`) in moment screens only: empty library, empty folder, no search results, empty Deleted items, nothing shared yet, the 404 page, first-run setup, the sign-in side panel on wide screens, and success modals. Never in the header, sidebar, cards, dashboards or settings. One character per screen at most, behind the main action. WebP at 2× display size, lazy-loaded, `alt=""`, a soft light backdrop so the dark robe reads on the dark theme. Security emails stay plain; a welcome email may carry the small avatar badge, embedded in the email rather than linked.

4. **Structured logs** (S–M). Write activity and errors as structured JSON (`LOG_FORMAT=json`), to stdout and optionally to a file, with a request ID on every line and in an `X-Request-Id` response header. Today logs are plain text on stdout only, which makes self-hosted problems hard to report and debug.

---

## Next: high value after release

5. **Fast capture** (M). Saving a page is the core action of a bookmark manager, and today it means opening Linkdean and pasting a URL.
   - Browser extension: save the current tab, and file it to a folder from the popup. Uses an API key.
   - PWA share target, so "Share → Linkdean" works on phones once the app is installed.
   - Bookmarklet as a zero-install fallback.

6. **Dashboard upgrades** (M). Make boards more useful at a glance.
   - Choose how each item displays: a full bookmark card in small, medium or large, or icons only.
   - Dynamic sections that fill themselves: the latest bookmarks, and the bookmarks you interact with most.

7. **API platform** (M). Rate limits, API keys and Prometheus metrics already ship. Fill the remaining gaps so the API can be built on:
   - An OpenAPI spec generated from the routes, published as the API reference.
   - One error body shape (`code`, `message`, details) across every route.
   - `ETag` / `Cache-Control` on read endpoints that are safe to cache.
   - Optional OpenTelemetry tracing, tied to the request ID from item 4.
   - API tests per route for auth, authorization and error responses.

8. **Privacy-respecting usage statistics** (M). Know how a public instance is used without tracking people.
   - Aggregate counts only (active users, bookmarks saved, features used), no personal data, page content or URLs, and nothing per person.
   - Off by default on self-hosted instances. Operators can send the statistics to their own backend with their own key (an OpenTelemetry exporter, or a service such as Datadog), building on the tracing in item 7.
   - The managed service uses the same hooks with its own backend.
   - Document exactly what is collected.

9. **Documentation site** (M). A [Docusaurus](https://docusaurus.io/) site for people who find the project after release, built from the existing docs. Not needed until the app is closer to production:
   - Self-hosting, with every environment variable and what it does.
   - Development setup.
   - Features list.
   - API reference, from the OpenAPI spec in item 7.
   - FAQ and code examples.

10. **More sign-in options** (M).
    - Passkeys / WebAuthn.
    - Preset SSO providers: Authentik, Google, Microsoft, GitHub. OIDC with any standard issuer already works; GitHub needs OAuth, not OIDC.

11. **Admin-set upload limits** (S). Let admins set the maximum size of uploaded images (avatars, previews) in platform settings, below the built-in hard caps.

---

## Later

12. **Archive & downloads** (L). Protects against link rot, which is the main reason people keep bookmarks in a self-hosted app. Heavy infrastructure.
    - Opt-in local cache of bookmarked pages.
    - Download / export: PDF, Markdown (reader mode), images, audio, video.
    - Separate storage volume and a background job queue.
    - Deploy: archive worker and volume mount convention.

13. **Organizations and teams** (L). Organizations own shared content in team spaces, with owner, admin, member and guest roles, team roles, restricted folders, email invites, lifecycle rules and a basic audit log. Personal libraries stay private and unchanged. Requirements: [organizations and teams design](./superpowers/specs/2026-11-03-organizations-and-teams-design.md). Needs outbound email parts B and C first. After v1:
    - Invite links (role, expiry, use limit; off by default).
    - Verified domains (DNS TXT proof), then auto-join for matching verified addresses.
    - SSO (SAML / per-organization OIDC) and SCIM provisioning.
    - Team vaults: shared keys, rotation when members leave, recovery.
    - Audit log: every content edit and restricted-folder view, configurable retention, SIEM or webhook streaming, hash-chained tamper evidence.
    - Custom roles and a Billing manager role, when a customer needs them.

14. **Localization** (M). i18n-ready strings first, then locale packs.

15. **Test coverage** (ongoing). Raise coverage on the API routes and the web pages with the least tests. Every fix and feature already ships with tests.

---

## Far out

Revisit once the stages above are done.

- **Client-held vault key** (L). True end-to-end encryption, where the server never sees the key. Makes vault search much harder.
- **Managed service** (L). Linkdean stays self-hosted first, with an optional hosted offering:
  - Hosted on a cloud provider (GCP, AWS or similar), starting as a thin, cheap layer that scales up with demand.
  - A marketing landing page: what the app is, links to the docs and GitHub, membership tiers and pricing.
  - Plans as set out in the organizations and teams design (Free, Pro, Team, Business, Enterprise; self-hosted ungated). Decide pricing models and whether there are one or several paid personal plans.
- **Native mobile apps** (L). iOS and Android, separate from the responsive web app and PWA.
- **Desktop app** (M). A [Tauri](https://tauri.app/) wrapper around the web app for macOS, Windows and Linux.
- **Gamification** (M). Points and badges on top of likes and ratings.
- **Previews for sites behind bot protection** (M). Some sites, such as those behind Cloudflare, block the screenshot and metadata fetch. Investigate allowed options, such as falling back to the site's own OG data or a user-uploaded preview.
- **Easter eggs** (S). Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), which needs React 19 support first).
