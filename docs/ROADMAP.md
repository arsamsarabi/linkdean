# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`. None of them block a public release today.

The first public release is a **free beta on the homelab** (`app.linkdean.app`). A cheap VPS and a full GCP deployment stay Far out; see [MANAGED_SERVICE.md](./MANAGED_SERVICE.md).

---

## Now: before the first public release

1. **Security hardening** (M). A public instance draws scanners and attackers from day one. Session and OIDC cookies are already `HttpOnly` and `SameSite`; image routes send `nosniff` and a sandbox CSP. **Done:** app-wide security headers (CSP with `frame-ancestors 'none'`, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`, `X-Frame-Options`; optional HSTS via `HSTS_MAX_AGE`), `/.well-known/security.txt`, CI `cargo audit` and `pnpm audit --prod`, and Dependabot for Cargo, npm and Actions.
   - Still open: a security review before launch — auth and session flows, sharing and public-list access checks, the page fetch and screenshot path (SSRF), import parsing, and upload handling.

2. **Sign-up protection** (S–M). **Done:** public beta keeps **open registration**. Misuse controls: optional Cloudflare Turnstile on password registration (`TURNSTILE_*`), host-forced verified-email gate (`REQUIRE_VERIFIED_EMAIL`, ORs with the admin toggle), and optional disposable-domain blocking (`BLOCK_DISPOSABLE_EMAILS`). Invite-only remains an admin toggle. Registration rate limits and email verification were already in place.

3. **Per-user quotas** (M). **Done:** platform-default limits (null = unlimited) for bookmarks, stored image bytes, enrich actions per UTC day, import bytes, and upload image bytes. Enforced at create/import/upload/enrich; `resolve_limits` + period-keyed usage leave room for membership plans. Usage in Settings when any limit is set. No per-user overrides in v1.

4. **Page the library list** (L). **Done:** `GET /bookmarks` returns a keyset page (`items`, `next_cursor`, `total`) with folder / tag / flag / since / sort filters; `GET /bookmarks/ids` backs select-all. The SPA uses infinite scroll; Explorer loads the open folder's page. Remaining: push the keyset into SQL (today the filtered set is paged in memory — see `bookmark_page.rs`), and restore Explorer folder counts without a full fetch.

5. **Account deletion and full export** (M). **Done:** Settings → General danger zone schedules deletion (email confirm + password when local; 7-day grace; cancel anytime before purge). Hourly maintenance hard-deletes the user row (FK cascade) and cleans Meili + preview assets. Export includes vault items as plaintext while the vault is unlocked. Owned shared folders are deleted (no hand-off). Remaining: fresh OIDC step-up for SSO-only accounts; last-admin check at purge as well as schedule.

6. **Legal pages and beta notice** (S). A public service needs terms people agree to before sign-up.
   - Terms of service, privacy policy (what is stored, where, for how long, who processes it, such as the email provider and Cloudflare) and an acceptable use policy.
   - Sign-up records which version of the terms was accepted, and when.
   - Only strictly necessary cookies are used today, so no consent banner is needed; say so in the privacy policy, and revisit if analytics are added.
   - A visible beta notice: no uptime promise, export your data regularly.

7. **Report abuse** (M). Public lists, profiles and comments are visible to everyone, and admins can ban users today, but nobody can report content.
   - A **Report** action on public lists, profiles and comments, with a reason.
   - A report queue in the admin moderation tools, with actions to hide the content, warn or ban, and dismiss.
   - An abuse and takedown contact address on the marketing site and in the terms.

8. **Final UI/UX and mobile pass** (L). Before the public release, review the whole UI with a strong UI/UX skill and reimagine it where needed. Make every page responsive and usable on phones, not only the nav.
   - Mascot art (`brand/graphics`) in moment screens only: empty library, empty folder, no search results, empty Deleted items, nothing shared yet, the 404 page, first-run setup, the sign-in side panel on wide screens, and success modals. Never in the header, sidebar, cards, dashboards or settings. One character per screen at most, behind the main action. WebP at 2× display size, lazy-loaded, `alt=""`, a soft light backdrop so the dark robe reads on the dark theme. Security emails stay plain; a welcome email may carry the small avatar badge, embedded in the email rather than linked.

9. **Structured logs** (S–M). Write activity and errors as structured JSON (`LOG_FORMAT=json`), to stdout and optionally to a file, with a request ID on every line and in an `X-Request-Id` response header. Today logs are plain text on stdout only, which makes self-hosted problems hard to report and debug.

10. **Marketing site** (M). Right after the product is ready for a public beta. A static site at `linkdean.app`: what Linkdean is, features, screenshots, a "self-host it for free" path to the install guide and GitHub, sign-up, and links to the legal pages and status.
    - Static and cached at Cloudflare, so it costs the server nothing.
    - Keep it in this repository (for example `apps/site`) so features and docs stay in step.

11. **Documentation site** (M). Immediately after the marketing site. A [Docusaurus](https://docusaurus.io/) site at `docs.linkdean.app`, built from the existing docs. Needed at launch so public users and self-hosters have one place to read:
    - Self-hosting, with every environment variable and what it does.
    - Development setup.
    - Features list.
    - API reference, from the OpenAPI spec in **API platform** (added once that ships).
    - FAQ and code examples.

12. **Membership tiers and pricing** (M, explore). Right after the marketing and documentation sites. Decide what the hosted service charges for, before anyone pays and before Stripe or plan gates land in code.
    - Start from the plan sketch in the [organizations and teams design](./superpowers/specs/2026-11-03-organizations-and-teams-design.md) (Free, Pro, Team, Business, Enterprise; self-hosted ungated).
    - Decide: free beta only for now, or published prices with charging deferred; one paid personal plan or several; what each plan's limits are (bookmarks, storage, screenshots, seats).
    - Write the model into [MANAGED_SERVICE.md](./MANAGED_SERVICE.md) and the marketing site's pricing section. Implementing Stripe and plan checks stays with a later paid launch, not the free homelab beta.

13. **Public beta on the homelab** (S). Run the public instance at `app.linkdean.app` as a stand-alone stack on the home server, with the marketing and documentation sites beside it. Plan and checklist: [MANAGED_SERVICE.md](./MANAGED_SERVICE.md#public-beta-on-a-home-server).
    - Isolated Compose project (own networks, volumes and secrets; preferably its own VM), Cloudflare Tunnel instead of open ports, a firewall rule keeping Chrome off the LAN, a transactional email provider, off-site backups with a tested restore, and an external uptime check.

---

## Next: high value after release

14. **Fast capture** (M). Saving a page is the core action of a bookmark manager, and today it means opening Linkdean and pasting a URL.
    - Browser extension: save the current tab, and file it to a folder from the popup. Uses an API key.
    - PWA share target, so "Share → Linkdean" works on phones once the app is installed.
    - Bookmarklet as a zero-install fallback.

15. **Auto-fill when adding a bookmark** (M). Title, description and images are fetched only after a bookmark is saved, so the form starts empty and people type a title the server would have found anyway.
    - An **Auto-fill** button below the URL field in the bookmark drawer, disabled until the URL is valid. It asks the API to fetch the page and fills the empty fields: title, description, and suggested tags if the page has keywords. Never overwrite what the person already typed.
    - Loading state on the button and fields while it runs, and an inline error when the page cannot be fetched or times out; the form stays usable either way.
    - The API reuses the worker's page fetch (plain HTTP and Open Graph first, headless Chrome only when the page needs JavaScript), with the same private-address blocking, a per-user rate limit and a short timeout.
    - Vault bookmarks are never enriched today, so the URL never leaves the instance for them. Keep the button hidden or behind an explicit confirmation when the bookmark is going into the vault.

16. **Create a category from the bookmark form** (S). Filing a bookmark into a folder that does not exist yet means closing the drawer, creating the category, and starting again.
    - Let the Categories field in the bookmark drawer create one: typing a name that matches nothing offers **Create "name"**, which creates it and selects it.
    - Choose where it goes: at the top level by default, or under a parent picked inline.
    - Only offered where the person can write, so not inside read-only shared folders.

17. **Duplicates on import** (M, explore first). Import adds every bookmark it reads, even when the library already has that URL, so importing the same file twice doubles the library. Let the person decide instead.
    - Before anything is written, warn that the file has bookmarks whose URLs are already in the library, with the count.
    - For each duplicate, show the existing bookmark next to the one being imported (title, URL, folder, tags, notes) and ask: **Replace** the existing one, **Skip** the imported one, or **Import anyway** as a second copy.
    - Apply-to-all shortcuts for large imports, so nobody clicks through hundreds of rows.
    - To explore: what counts as the same URL (Library health → Duplicates matches exact URLs only, so `https://a.com` and `https://a.com/` differ); whether Replace keeps the existing bookmark's ID, shares, pins and activity or swaps them for the imported one; vault bookmarks; and how the review step works with the import API, which today runs in one request.

18. **Dashboard upgrades** (M). Make boards more useful at a glance.
    - Choose how each item displays: a full bookmark card in small, medium or large, or icons only.
    - Dynamic sections that fill themselves: the latest bookmarks, and the bookmarks you interact with most.

19. **More sign-in options** (M).
    - Passkeys / WebAuthn.
    - Preset SSO providers: Authentik, Google, Microsoft, GitHub. OIDC with any standard issuer already works; GitHub needs OAuth, not OIDC.

20. **Retry failed screenshots** (S). A screenshot that times out (common right after a large import, when Chrome gets a burst of pages) leaves the bookmark on its Open Graph image or favicon. Today the only retry is automatic: when the API starts, it retries failures older than 7 days, so in practice they wait for the next restart.
    - A **Retry failed screenshots** button on the admin settings page, with the number of bookmarks that would be retried. It queues them through the existing backfill, which runs two screenshots at a time.
    - A cooldown between retries so Chrome and the sites it visits are not hammered: the button is disabled, showing when it is next available, until the cooldown has passed since the last run. Default 24 hours, admin-configurable in platform settings.
    - Retry only bookmarks whose last failure is older than the cooldown, so a manual run never retries a page that just failed.
    - Make the 7-day automatic retry window admin-configurable in the same place, and run it on a schedule instead of only at API start.

21. **API platform** (M). Rate limits, API keys and Prometheus metrics already ship. Fill the remaining gaps so the API can be built on:
    - An OpenAPI spec generated from the routes, published as the API reference.
    - One error body shape (`code`, `message`, details) across every route.
    - `ETag` / `Cache-Control` on read endpoints that are safe to cache.
    - Optional OpenTelemetry tracing, tied to the request ID from **Structured logs**.
    - API tests per route for auth, authorization and error responses.

22. **Privacy-respecting usage statistics** (M). Know how a public instance is used without tracking people.
    - Aggregate counts only (active users, bookmarks saved, features used), no personal data, page content or URLs, and nothing per person.
    - Off by default on self-hosted instances. Operators can send the statistics to their own backend with their own key (an OpenTelemetry exporter, or a service such as Datadog), building on the tracing in **API platform**.
    - The managed service uses the same hooks with its own backend.
    - Document exactly what is collected.

---

## Later

23. **Archive & downloads** (L). Protects against link rot, which is the main reason people keep bookmarks in a self-hosted app. Heavy infrastructure.
    - Opt-in local cache of bookmarked pages.
    - Download / export: PDF, Markdown (reader mode), images, audio, video.
    - Separate storage volume and a background job queue.
    - Deploy: archive worker and volume mount convention.

24. **Notification digests and one-click unsubscribe (`List-Unsubscribe`)** (S–M). Notification emails go out one at a time today, so the noisy kinds (edits, likes, comments) are off by default, and **Settings → Notifications** is the only way to stop them.
    - A daily digest for the noisy kinds.
    - A one-click unsubscribe link per kind in every notification email, with the `List-Unsubscribe` and `List-Unsubscribe-Post` headers.

25. **Organizations and teams** (L). Organizations own shared content in team spaces, with owner, admin, member and guest roles, team roles, restricted folders, email invites, lifecycle rules and a basic audit log. Personal libraries stay private and unchanged. Requirements: [organizations and teams design](./superpowers/specs/2026-11-03-organizations-and-teams-design.md). Phases: [organizations and teams phases](./superpowers/plans/2026-11-03-organizations-and-teams-phases.md). After v1:
    - Invite links (role, expiry, use limit; off by default).
    - Verified domains (DNS TXT proof), then auto-join for matching verified addresses.
    - SSO (SAML / per-organization OIDC) and SCIM provisioning.
    - Team vaults: shared keys, rotation when members leave, recovery.
    - Audit log: every content edit and restricted-folder view, configurable retention, SIEM or webhook streaming, hash-chained tamper evidence.
    - Custom roles and a Billing manager role, when a customer needs them.

26. **Localization** (M). i18n-ready strings first, then locale packs.

27. **Test coverage** (ongoing). Raise coverage on the API routes and the web pages with the least tests. Every fix and feature already ships with tests.

---

## Far out

Revisit once the stages above are done.

- **AI features** (?). To explore: which features (for example auto-tagging, summaries, semantic search), a free tier with limited usage, and bring-your-own-key (BYOK), possibly for a discount on paid plans. Idea from `docs/IDEAS.md`.
- **Client-held vault key** (L). True end-to-end encryption, where the server never sees the key. Makes vault search much harder.
- **Cheap VPS** (S–M). Move the same Compose stack off the homelab when outages, upload bandwidth, CPU, or abuse on the home connection start to matter, or when charging begins. Same [INSTALL.md](./INSTALL.md); no multi-instance code changes. Triggers and notes: [MANAGED_SERVICE.md](./MANAGED_SERVICE.md#when-to-move-off-the-home-server).
- **GCP deployment** (L). Multi-instance managed hosting when one server is no longer enough: Cloud Run, Cloud SQL, object storage, shared rate limits, worker split, Terraform, Stripe and plan gates. Full plan: [MANAGED_SERVICE.md](./MANAGED_SERVICE.md#moving-to-gcp).
- **Native mobile apps** (L). iOS and Android, separate from the responsive web app and PWA.
- **Desktop app** (M). A [Tauri](https://tauri.app/) wrapper around the web app for macOS, Windows and Linux.
- **Gamification** (M). Points and badges on top of likes and ratings.
- **Previews for sites behind bot protection** (M). Some sites, such as those behind Cloudflare, block the screenshot and metadata fetch. Investigate allowed options, such as falling back to the site's own OG data or a user-uploaded preview.
- **Easter eggs** (S). Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), which needs React 19 support first).
