# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`. None of them block a public release today.

The first public release is a **free beta on the homelab** (`app.linkdean.app`). A cheap VPS and a full GCP deployment stay Far out; see [MANAGED_SERVICE.md](./MANAGED_SERVICE.md).

---

## Now: before the first public release

2. **Sign-up protection** (S–M). **Done:** public beta keeps **open registration**. Misuse controls: optional Cloudflare Turnstile on password registration (`TURNSTILE_*`), host-forced verified-email gate (`REQUIRE_VERIFIED_EMAIL`, ORs with the admin toggle), and optional disposable-domain blocking (`BLOCK_DISPOSABLE_EMAILS`). Invite-only remains an admin toggle. Registration rate limits and email verification were already in place.

3. **Per-user quotas** (M). **Done:** platform-default limits (null = unlimited) for bookmarks, stored image bytes, enrich actions per UTC day, import bytes, and upload image bytes. Enforced at create/import/upload/enrich; `resolve_limits` + period-keyed usage leave room for membership plans. Usage in Settings when any limit is set. No per-user overrides in v1.

4. **Page the library list** (L). **Done:** `GET /bookmarks` returns a keyset page (`items`, `next_cursor`, `total`) with folder / tag / flag / since / sort filters; `GET /bookmarks/ids` backs select-all. The SPA uses infinite scroll; Explorer loads the open folder's page. Remaining: push the keyset into SQL (today the filtered set is paged in memory — see `bookmark_page.rs`), and restore Explorer folder counts without a full fetch.

5. **Account deletion and full export** (M). **Done:** Settings → General danger zone schedules deletion (email confirm + password when local; 7-day grace; cancel anytime before purge). Hourly maintenance hard-deletes the user row (FK cascade) and cleans Meili + preview assets. Export includes vault items as plaintext while the vault is unlocked. Owned shared folders are deleted (no hand-off). Remaining: fresh OIDC step-up for SSO-only accounts; last-admin check at purge as well as schedule.

6. **Legal pages and beta notice** (S). **Done:** draft Terms, Privacy and Acceptable Use pages (`/terms`, `/privacy`, `/acceptable-use`), required terms checkbox at sign-up and for SSO start (version + time stored on new accounts), and a dismissible beta banner. Remaining: counsel review of the copy before charging money; re-accept flow when `TERMS_VERSION` bumps.

7. **Report abuse** (M). **Done:** signed-in users can **Report** a public list, a profile or a comment with a reason; admins work the queue under Admin (dismiss, hide content, warn, ban); the operator's abuse contact email is an admin setting shown in the report form; marketing footer lists `contact@linkdean.app`. Remaining: anonymous reports (out of scope for spam control).

8. **Final UI/UX and mobile pass** (L). **Done:** mascot (Dean) on moment empty states, 404, onboarding and the wide sign-in panel; drawers full-width on phones; tighter shell padding and scrollable Deleted table; light polish on dashboard empty copy and glance density. Remaining: welcome-email avatar badge when a welcome template exists (security emails stay plain); deeper visual redesign stays out of scope for beta.

9. **Structured logs** (S–M). **Done:** `LOG_FORMAT=text|json` (default text) for stdout; optional `LOG_FILE` appends JSONL regardless of stdout format; every response has `X-Request-Id` (valid inbound ids kept, else a UUID), with the same id on the HTTP request span. Remaining: log rotation stays with the operator; OpenTelemetry exporters stay in Backlog.

10. **Marketing site** (M). **Done:** Astro site in `apps/site`, live at [linkdean.app](https://linkdean.app) via `deploy/public-beta/static/site` — product pitch, features, self-host CTA to the docs site, sign-up / sign-in, legal + GitHub links, `contact@linkdean.app`. Static and cached at Cloudflare. Remaining: pricing section waits on item 12.

11. **Documentation site** (M). **Done:** [Docusaurus](https://docusaurus.io/) at [`apps/docs`](../apps/docs), live at [docs.linkdean.app](https://docs.linkdean.app). Builds from repo `docs/` (Install, Operations / development setup, Features, Architecture) via `pnpm sync` + `pnpm build` into `deploy/public-beta/static/docs`. Remaining: API reference once **API platform** ships OpenAPI; optional FAQ pages.

12. **Membership tiers and pricing** (M, explore). After go-live hardening (#13), before anyone pays and before Stripe or plan gates land in code.
   - Start from the plan sketch in the [organizations and teams design](./superpowers/specs/2026-11-03-organizations-and-teams-design.md) (Free, Pro, Team, Business, Enterprise; self-hosted ungated).
   - Decide: free beta only for now, or published prices with charging deferred; one paid personal plan or several; what each plan's limits are (bookmarks, storage, screenshots, seats).
   - Write the model into [MANAGED_SERVICE.md](./MANAGED_SERVICE.md) and the marketing site's pricing section. Implementing Stripe and plan checks stays with a later paid launch, not the free homelab beta.

13. **Public beta on the homelab** (S). **In progress:** stand-alone LXD guest on Nova (`deploy/public-beta/`), Cloudflare Tunnel for `app.linkdean.app`, `docs.linkdean.app`, and apex/www. App, docs, and marketing are live. Guest `.env` SoT is Infisical `/linkdean-public`; off-site backups are restic → B2 bucket `linkdean-public` ([BACKUP.md](../deploy/public-beta/BACKUP.md)). Checklist: [MANAGED_SERVICE.md](./MANAGED_SERVICE.md#public-beta-on-a-home-server). Design: [public-beta-homelab](./superpowers/specs/2026-11-06-public-beta-homelab-design.md), [restic-infisical](./superpowers/specs/2026-11-07-public-beta-restic-infisical-design.md).
   - Go-live checklist. **Done** 2026-11-09: sign-up controls checked on the live beta with `deploy/public-beta/scripts/check-signup-controls.sh` (Turnstile, disposable-domain block, outbound mail) and `REQUIRE_VERIFIED_EMAIL=true` confirmed in Infisical; first restic restore drill passed on Nova ([drill log](../deploy/public-beta/BACKUP.md#drill-log)). External uptime monitoring moved to Later (#28).
---

## Next: high value after release

14. **Fast capture, with auto-fill** (M–L). Saving a page is the core action of a bookmark manager, and today it means opening Linkdean and pasting a URL, then typing a title the server would have found anyway. Absorbs the former item 15 (Auto-fill). Built in slices, each shippable:
    - **Slice 0, page details from a URL.** **Done:** `POST /api/bookmarks/page-details` and the **Import from URL** button on the bookmark form (see FEATURES). The fetch is plain HTTP only, never Chrome, so pages that build their title in JavaScript come back empty.
    - **Slice 1, quick save page.** **Done:** `/save` (see FEATURES). URL matching is exact, so `https://a.com` and `https://a.com/` count as different pages, as in Library health → Duplicates.
    - **Slice 2, bookmarklet.** **Done:** **Settings → Fast capture** (see FEATURES).
    - **Slice 3, share target.** **Done:** `share_target` in `site.webmanifest`, and the iOS Shortcuts in [Saving pages fast](./FAST_CAPTURE.md) (see FEATURES).
    - **Slice 4, browser extension.** **Submitted to both stores, in review:** `apps/extension` 0.1.0 (see FEATURES). The Release workflow submits later versions on a tag. Remaining: approval, then the store links in the web app and docs.

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

28. **External uptime monitoring for the public beta** (S). Nothing outside the homelab notices when Nova, the guest or the home connection goes down, so an outage lasts until someone happens to look.
    - Set up the checks in [MONITORING.md](../deploy/public-beta/MONITORING.md): `/api/ready` keyword check as the main one, plus health, marketing and docs, every 5 minutes, alerting after 2 failures.
    - Push alerts to a phone app; email alone is too easy to miss.
    - Add Cloudflare's Tunnel Health Alert for the `linkdean-public` tunnel.
    - Test one alert end to end by stopping the api for 15 minutes in a quiet hour.

29. **Filing queue for Unsorted** (M). Fast capture (item 14) makes saving one tap, so links pile up in Unsorted with no folder, few tags and sometimes no title. A place to work through them one at a time turns that pile into a filed library.
    - A **File** view that steps through Unsorted bookmarks newest first, one card at a time: title, URL, preview, description.
    - Per card: pick a folder (or create one, see item 16), add tags, fix the title, mark favourite, or delete. **Next** saves and moves on; **Skip** leaves it in Unsorted.
    - Suggestions: folders and tags already used for bookmarks on the same site, and the page's keyword tags from item 14's fetch.
    - Keyboard first on desktop (number keys pick a suggested folder, Enter files, S skips, D deletes); swipe on phones.
    - A count of unsorted bookmarks on the sidebar's Unsorted entry, and an empty state when the queue is clear.
    - To explore: whether bulk filing (select several, one folder) belongs here or in the library list, and whether a bookmark counts as "filed" once it has tags but no folder.

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
