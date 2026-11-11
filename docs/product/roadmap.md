# Roadmap

What is still planned. Shipped capabilities live in [features.md](./features.md). Scratch ideas: [ideas.md](./ideas.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`.

The public beta is live on the homelab: the app at `app.linkdean.app`, docs at `docs.linkdean.app` and the marketing site at `linkdean.app`, with off-site backups and a passed restore drill ([BACKUP.md](../../deploy/public-beta/BACKUP.md)). A cheap VPS and a full GCP deployment stay Far out; see [managed-service.md](../ops/managed-service.md).

---

## Now: running the public beta well

1. **Browser extension in both stores** (S). The extension is the fastest way to save a page, but 0.1.0 is only live on the Chrome Web Store, and the store submission in the Release workflow has never run for real.
    a. **Arsam:** wait for AMO to approve 0.1.0 (email from Mozilla).
    b. **Prep** (needs a): bump `apps/extension/package.json` to 0.1.1, which ships the popup footer change from #94.
    c. **Arsam** (needs b): tag the next release. Its `extension` job is the first real run of the store submission; check its log.
    d. **Prep:** add the Chrome Web Store link to **Settings → Fast capture** and the docs now, and the AMO link once a is done.

2. **External uptime monitoring** (S). Nothing outside the homelab notices when Nova, the guest or the home connection goes down, so an outage on the live beta lasts until someone happens to look.
    - Set up the checks in [MONITORING.md](../../deploy/public-beta/MONITORING.md): `/api/ready` keyword check as the main one, plus health, marketing and docs, every 5 minutes, alerting after 2 failures.
    - Push alerts to a phone app; email alone is too easy to miss.
    - Add Cloudflare's Tunnel Health Alert for the `linkdean-public` tunnel.
    - Test one alert end to end by stopping the api for 15 minutes in a quiet hour.

3. **Filing queue for Unsorted** (M). Fast capture makes saving one tap, so links pile up in Unsorted with no folder, few tags and sometimes no title. A place to work through them one at a time turns that pile into a filed library.
    - A **File** view that steps through Unsorted bookmarks newest first, one card at a time: title, URL, preview, description.
    - Per card: pick a folder (or create one, as the bookmark form does), add tags, fix the title, mark favourite, or delete. **Next** saves and moves on; **Skip** leaves it in Unsorted.
    - Suggestions: folders and tags already used for bookmarks on the same site, and the page's keyword tags from the page-details fetch.
    - Keyboard first on desktop (number keys pick a suggested folder, Enter files, S skips, D deletes); swipe on phones.
    - A count of unsorted bookmarks on the sidebar's Unsorted entry, and an empty state when the queue is clear.
    - To explore: whether bulk filing (select several, one folder) belongs here or in the library list, and whether a bookmark counts as "filed" once it has tags but no folder.

4. **Library paging in SQL** (M). The library list pages with a keyset cursor, but the filtered set is still loaded and paged in memory, so large libraries cost more on every scroll as the beta grows.
    - Push the keyset and filters into the SQL query.
    - Restore Explorer folder counts without fetching every bookmark.

5. **Dashboard upgrades** (M). Make boards more useful at a glance.
    - Choose how each item displays: a full bookmark card in small, medium or large, or icons only.
    - Dynamic sections that fill themselves: the latest bookmarks, and the bookmarks you interact with most.

---

## Next: high value after the beta settles

6. **Notification digests and one-click unsubscribe (`List-Unsubscribe`)** (S–M). Notification emails go out one at a time today, so the noisy kinds (edits, likes, comments) are off by default, and **Settings → Notifications** is the only way to stop them.
    - A daily digest for the noisy kinds.
    - A one-click unsubscribe link per kind in every notification email, with the `List-Unsubscribe` and `List-Unsubscribe-Post` headers.

7. **Account and legal follow-ups** (S). Gaps left when account deletion and the legal pages shipped.
    - Fresh OIDC step-up before an SSO-only account schedules deletion.
    - Check for the last admin when the purge runs, not only when deletion is scheduled.
    - A re-accept flow when `TERMS_VERSION` changes.
    - Counsel review of the Terms, Privacy and Acceptable Use copy before charging money.

8. **More sign-in options** (M).
    - Passkeys / WebAuthn.
    - Preset SSO providers: Authentik, Google, Microsoft, GitHub. OIDC with any standard issuer already works; GitHub needs OAuth, not OIDC.

9. **API platform** (M). Rate limits, API keys and Prometheus metrics already ship. Fill the remaining gaps so the API can be built on:
    - An OpenAPI spec generated from the routes, published as the API reference on the docs site.
    - One error body shape (`code`, `message`, details) across every route.
    - `ETag` / `Cache-Control` on read endpoints that are safe to cache.
    - Optional OpenTelemetry tracing, tied to the request ID from the structured logs.
    - API tests per route for auth, authorization and error responses.

10. **Membership tiers and pricing** (M, explore). Before anyone pays and before Stripe or plan gates land in code.
    - Start from the plan sketch in the [organizations and teams design](../design/organizations-and-teams.md) (Free, Pro, Team, Business, Enterprise; self-hosted ungated).
    - Decide: free beta only for now, or published prices with charging deferred; one paid personal plan or several; what each plan's limits are (bookmarks, storage, screenshots, seats). Limits plug into the existing quota resolver.
    - Write the model into [managed-service.md](../ops/managed-service.md) and add a pricing section to the marketing site. Implementing Stripe and plan checks stays with a later paid launch, not the free homelab beta.

11. **Privacy-respecting usage statistics** (M). Know how a public instance is used without tracking people.
    - Aggregate counts only (active users, bookmarks saved, features used), no personal data, page content or URLs, and nothing per person.
    - Off by default on self-hosted instances. Operators can send the statistics to their own backend with their own key (an OpenTelemetry exporter, or a service such as Datadog), building on the tracing in **API platform**.
    - The managed service uses the same hooks with its own backend.
    - Document exactly what is collected.

---

## Later

12. **Archive & downloads** (L). Protects against link rot, which is the main reason people keep bookmarks in a self-hosted app. Heavy infrastructure.
    - Opt-in local cache of bookmarked pages.
    - Download / export: PDF, Markdown (reader mode), images, audio, video.
    - Separate storage volume and a background job queue.
    - Deploy: archive worker and volume mount convention.

13. **Organizations and teams** (L). Organizations own shared content in team spaces, with owner, admin, member and guest roles, team roles, restricted folders, email invites, lifecycle rules and a basic audit log. Personal libraries stay private and unchanged. Requirements: [organizations and teams design](../design/organizations-and-teams.md). After v1:
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

- **AI features** (?). To explore: which features (for example auto-tagging, summaries, semantic search), a free tier with limited usage, and bring-your-own-key (BYOK), possibly for a discount on paid plans. Idea from `docs/product/ideas.md`.
- **Client-held vault key** (L). True end-to-end encryption, where the server never sees the key. Makes vault search much harder.
- **Cheap VPS** (S–M). Move the same Compose stack off the homelab when outages, upload bandwidth, CPU, or abuse on the home connection start to matter, or when charging begins. Same [quick-start.md](../self-hosting/quick-start.md); no multi-instance code changes. Triggers and notes: [managed-service.md](../ops/managed-service.md#when-to-move-off-the-home-server).
- **GCP deployment** (L). Multi-instance managed hosting when one server is no longer enough: Cloud Run, Cloud SQL, object storage, shared rate limits, worker split, Terraform, Stripe and plan gates. Full plan: [managed-service.md](../ops/managed-service.md#moving-to-gcp).
- **Native mobile apps** (L). iOS and Android, separate from the responsive web app and PWA.
- **Desktop app** (M). A [Tauri](https://tauri.app/) wrapper around the web app for macOS, Windows and Linux.
- **Gamification** (M). Points and badges on top of likes and ratings.
- **Previews for sites behind bot protection** (M). Some sites, such as those behind Cloudflare, block the screenshot and metadata fetch. Investigate allowed options, such as falling back to the site's own OG data or a user-uploaded preview.
- **Welcome email** (S). A welcome email for new accounts, with Dean's avatar badge. Security emails stay plain.
- **Docs FAQ** (S). Frequently asked questions on the docs site.
- **Easter eggs** (S). Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), which needs React 19 support first).
