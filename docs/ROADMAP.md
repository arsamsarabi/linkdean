# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`. The ones that block a public release are item 1.

---

## Now: before the first public release

1. **Close the release-blocking known issues** (M). These let one person take over or break an instance, or leak vault plaintext:
   - One registration invite can create several accounts when registrations race (`consume_registration_invite`). Claim the invite and insert the user in one transaction.
   - Whoever registers first on a fresh install becomes admin, and two first registrations can both become admin (`register`). Add an advisory lock and an optional `BOOTSTRAP_TOKEN`.
   - Two admins demoting each other at once can leave no admin (`update_admin_user`). Check and update under the ban advisory lock.
   - Chrome 124 runs with `--no-sandbox`, and the dev stack does not isolate it from the database. Use a maintained image, the sandbox where possible, memory and pid limits, and the production networks in dev.
   - A vault move takes no row lock, so a concurrent edit can land as plaintext in a vault row (`write_encrypted`).
   - The session token in localStorage undoes the cookie's `HttpOnly`. Rely on the cookie in the browser.
   - No `.dockerignore`, so a local `node_modules` can override the image's.

2. **Outbound email** (M–L). SMTP config with [react-email](https://www.npmjs.com/package/react-email) templates. Unlocks the features below, which a public instance needs.
   - Password reset by email. There is no way to recover a forgotten password today short of an admin.
   - Verify email addresses at registration and on email change, before the address can receive share invites. See the known-issue note on `create_share` in `crates/db/src/repo/shares.rs`.
   - Until an address is verified, invites to it wait as pending invites for the address rather than going to the unverified account.
   - An approved link flow, so a local account with someone's email cannot block their first OIDC sign-in.
   - Email delivery for invites and notifications.

3. **Page the library list** (L). `GET /bookmarks` returns every bookmark, and the web app filters and sorts the whole set in the browser (`useLibraryList` → `filterAndSortLibraryItems`). Large libraries pay for that on every load, and an import of thousands of bookmarks makes the app slow from day one.
   - Use infinite scrolling, not numbered pages. The library is browsed and filtered rather than jumped through by page, and filters change the result set often.
   - Keyset pagination in the API (`cursor` + `limit`, ordered by the active sort key and `id`), so new or edited bookmarks do not shift or repeat rows the way `OFFSET` would.
   - Move folder, tag, flag, "since" and sort filters into the query, and return the total match count for headers and "select all".
   - Explorer loads one folder's contents at a time, rather than slicing the full list.
   - Bulk actions on "select all" act on the server-side match (filters sent to the API), not only the rows loaded so far. Keep H21's rule: select all reaches only what the open folder or search shows.
   - Vault search stays a Postgres scan of unlocked rows. Page it the same way.
   - Batch the access checks for bookmarks shared with the viewer, which still run one at a time.

4. **Final UI/UX and mobile pass** (L). Before the public release, review the whole UI with a strong UI/UX skill and reimagine it where needed. Make every page responsive and usable on phones, not only the nav.

5. **Structured logs** (S–M). Write activity and errors as structured JSON (`LOG_FORMAT=json`), to stdout and optionally to a file, with a request ID on every line and in an `X-Request-Id` response header. Today logs are plain text on stdout only, which makes self-hosted problems hard to report and debug.

---

## Next: high value after release

6. **Fast capture** (M). Saving a page is the core action of a bookmark manager, and today it means opening Linkdean and pasting a URL.
   - Browser extension: save the current tab, and file it to a folder from the popup. Uses an API key.
   - PWA share target, so "Share → Linkdean" works on phones once the app is installed.
   - Bookmarklet as a zero-install fallback.

7. **API platform** (M). Rate limits, API keys and Prometheus metrics already ship. Fill the remaining gaps so the API can be built on:
   - An OpenAPI spec generated from the routes, published as the API reference.
   - One error body shape (`code`, `message`, details) across every route.
   - `ETag` / `Cache-Control` on read endpoints that are safe to cache.
   - Optional OpenTelemetry tracing, tied to the request ID from item 5.
   - API tests per route for auth, authorization and error responses.

8. **Documentation site** (M). A [Docusaurus](https://docusaurus.io/) site for people who find the project after release, built from the existing docs. Not needed until the app is closer to production:
   - Self-hosting, with every environment variable and what it does.
   - Development setup.
   - Features list.
   - API reference, from the OpenAPI spec in item 7.
   - FAQ and code examples.

9. **More sign-in options** (M).
   - Passkeys / WebAuthn.
   - Preset SSO providers: Authentik, Google, Microsoft, GitHub. OIDC with any standard issuer already works; GitHub needs OAuth, not OIDC.

10. **Admin-set upload limits** (S). Let admins set the maximum size of uploaded images (avatars, previews) in platform settings, below the built-in hard caps.

---

## Later

11. **Archive & downloads** (L). Protects against link rot, which is the main reason people keep bookmarks in a self-hosted app. Heavy infrastructure.
    - Opt-in local cache of bookmarked pages.
    - Download / export: PDF, Markdown (reader mode), images, audio, video.
    - Separate storage volume and a background job queue.
    - Deploy: archive worker and volume mount convention.

12. **Teams** (L). Shared spaces for households and small groups, beyond sharing single folders.
    - Create a team; invite and remove members.
    - View a team, its members, bookmarks, lists and dashboards.
    - Team activity, comments and likes.

13. **Localization** (M). i18n-ready strings first, then locale packs.

14. **Test coverage** (ongoing). Raise coverage on the API routes and the web pages with the least tests. Every fix and feature already ships with tests.

---

## Far out

Revisit once the stages above are done.

- **Client-held vault key** (L). True end-to-end encryption, where the server never sees the key. Makes vault search much harder.
- **Managed service** (L). Linkdean stays self-hosted first, with an optional hosted offering:
  - Hosted on a cloud provider (GCP, AWS or similar), starting as a thin, cheap layer that scales up with demand.
  - A marketing landing page: what the app is, links to the docs and GitHub, membership tiers and pricing.
- **Native mobile apps** (L). iOS and Android, separate from the responsive web app and PWA.
- **Gamification** (M). Points and badges on top of likes and ratings.
- **Previews for sites behind bot protection** (M). Some sites, such as those behind Cloudflare, block the screenshot and metadata fetch. Investigate allowed options, such as falling back to the site's own OG data or a user-uploaded preview.
- **Easter eggs** (S). Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), which needs React 19 support first).
