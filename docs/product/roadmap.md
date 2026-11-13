# Roadmap

What is still planned. Shipped capabilities live in [features.md](./features.md). Scratch ideas: [ideas.md](./ideas.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

The public [roadmap page](../../apps/docs/docs/roadmap.md) on the docs site is a plain-language copy of this list, without the internal steps. Update it in the same commit when an item is added, moved or shipped.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`.

The public beta is live on the homelab: the app at `app.linkdean.app`, docs at `docs.linkdean.app` and the marketing site at `linkdean.app`, with off-site backups and a passed restore drill ([BACKUP.md](../../deploy/public-beta/BACKUP.md)). A cheap VPS and a full GCP deployment stay Far out; see [managed-service.md](../ops/managed-service.md).

---

## Now: running the public beta well

1. **Browser extension in both stores** (S). The extension is the fastest way to save a page, but 0.1.0 is only live on the Chrome Web Store, and the store submission in the Release workflow has never run for real.
    a. **Arsam:** wait for AMO to approve 0.1.0 (email from Mozilla).
    b. **Prep** (needs a): bump `apps/extension/package.json` to 0.1.1, which ships the popup footer change from #94.
    c. **Arsam** (needs b): tag the next release. Its `extension` job is the first real run of the store submission; check its log.
    d. **Prep** (needs a): add the AMO link beside the Chrome Web Store link in **Config options → Fast capture** and the docs. The Chrome link is in.

5. **Dashboard upgrades** (M). Make boards more useful at a glance.
    - Choose how each item displays: a full bookmark card in small, medium or large, or icons only.
    - Dynamic sections that fill themselves: the latest bookmarks, and the bookmarks you interact with most.

---

## Next: high value after the beta settles

Archiving and AI assistance come first: they are the biggest gaps against Raindrop.io, Linkwarden and Karakeep on the public [comparison page](../../apps/docs/docs/compare.md).

12. **Archive & downloads** (L, in phases). Link rot is the main reason people keep bookmarks on their own server, and Linkwarden, Karakeep and Raindrop.io (Pro) all keep a copy of each page. Linkdean stores previews only.
    - Phase 1, snapshots: when a bookmark is saved, the worker's existing headless Chrome also captures a single-file HTML copy and a PDF. Opt-in per account; stored on a separate archive volume and counted against the storage quota.
    - Phase 2, page text: reader-mode text taken from the snapshot and indexed in Meilisearch, so search covers page contents, not only titles and notes.
    - Downloads: per bookmark, download the copy as HTML, PDF or reader-mode Markdown. The full export gains an option to include archives (zip).
    - Dead links: when the broken-link check finds a page gone, the bookmark offers its archived copy. Re-archive on demand.
    - Vaults: archived copies of vault bookmarks are encrypted with the vault key and never indexed.
    - Media (images, audio, video) last, behind a size cap per item and per account.
    - Deploy: archive volume and mount convention, a worker concurrency limit, the archive volume in the restic backup, self-host settings to turn it off or cap it.
    - Open questions: on by default for new accounts or opt-in; retention for deleted bookmarks; quota on the hosted beta.

16. **AI assistance** (M–L). Karakeep is built around AI tagging, Linkwarden offers it with local models, and Raindrop.io has it on Pro. Linkdean has none. Add it without giving up the privacy stance.
    - Off by default. Each account opts in; the operator chooses the provider: bring-your-own-key (any OpenAI-compatible API, or Anthropic) or a local model (Ollama) on a self-hosted instance.
    - Workflows, in order: tag and folder suggestions when saving and in **File one by one** (extends the same-site suggestions); a short summary per bookmark (from the archived page text when there is one); semantic search ("that article about...") through Meilisearch hybrid search with embeddings; related-bookmark and near-duplicate hints.
    - Suggestions, not silent changes: nothing is applied without a click, unless the person turns on auto-tagging.
    - Vault contents never leave the server for a model. The settings page shows what is sent.
    - Hosted service: a limited free allowance, more with your own key (ties into item 10).
    - Depends on: phase 2 of item 12 for summaries and semantic search over page contents; tag suggestions do not.

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

10. **Membership tiers and pricing for managed services** (M, explore). Before anyone pays and before Stripe or plan gates land in code.
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
