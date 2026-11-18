# Roadmap

What is still planned. Shipped capabilities live in [features.md](./features.md). Scratch ideas: [ideas.md](./ideas.md).

Items are ranked by importance and by the value they add to the app, highest first, within each stage. Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks.

The public [roadmap page](https://docs.linkdean.app/docs/roadmap) on the docs site is a plain-language copy of this list, without the internal steps. Update it when an item is added, moved or shipped.

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`.

---

## Now: running the public beta well

Nothing open right now. The next item comes from **Next** below.

---

## Next: high value after the beta settles

AI assistance comes first among the remaining gaps against Raindrop.io, Linkwarden and Karakeep on the public [comparison page](https://docs.linkdean.app/docs/compare). Archive page-text search has shipped; Markdown download and the rest of item 12 are still open.

12. **Archive & downloads** (L, in phases). Shipped so far: snapshots as HTML, PDF or the file itself, the viewer, downloads, vault encryption, the dead-link and Quick peek fallbacks, admin limits, and page-text search from non-vault HTML archives via Meilisearch (see [features](./features.md#archive) and the [archive design](../design/archive.md)). Still to come:
    - Downloads: reader-mode Markdown per bookmark, and an option in the full export to include archives (zip).
    - Media: explore downloading embedded `<video>` and `<audio>` files that are plain links, not streams, behind the per-item and per-account caps.
    - Version history: keep more than one copy per format.
    - Tiers: per-tier archive storage limits for the hosted service.
    - Archive imported bookmarks when **Archive new bookmarks** is on, throttled.
    - Hide the archive actions when an admin has turned archiving off (today the server refuses with a message).

16. **AI assistance** (M–L). Karakeep is built around AI tagging, Linkwarden offers it with local models, and Raindrop.io has it on Pro. Linkdean has none. Add it without giving up the privacy stance.
    - Off by default. Each account opts in; the operator chooses the provider: bring-your-own-key (any OpenAI-compatible API, or Anthropic) or a local model (Ollama) on a self-hosted instance.
    - Workflows, in order: tag and folder suggestions when saving and in **File one by one** (extends the same-site suggestions); a short summary per bookmark (from the archived page text when there is one); semantic search ("that article about...") through Meilisearch hybrid search with embeddings; related-bookmark and near-duplicate hints.
    - Suggestions, not silent changes: nothing is applied without a click, unless the person turns on auto-tagging.
    - Vault contents never leave the server for a model. The settings page shows what is sent.
    - Hosted service: a limited free allowance, more with your own key.
    - Depends on: phase 2 of item 12 for summaries and semantic search over page contents; tag suggestions do not.
    - Later: AI-assisted dashboards. Suggest sections and their sources (folders, tags, websites) from what the person saves and opens, built on the sections that fill themselves.

22. **Agent Skills, next steps** (S–M). Folders and tags already download as Agent Skills, and an API key can fetch them (see features).
    - A read-only MCP server (search and read bookmarks), as well as or instead of generated files.
    - With item 12, include the archived reader-mode text so the agent can read a page without fetching it.
    - Skills from a saved library search, once saved searches exist.

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

15. **Test coverage** (ongoing). Raise coverage on the API routes and the web pages with the least tests. Every fix and feature already ships with tests. Playwright `@smoke` and most `@p1` already run on the throwaway stack in PR CI; remaining journey stubs stay in `apps/web/e2e/`:
    - Enable **LIB-02** (invalid URL on blur blocks Add). The drawer now validates; confirm smoke, then flip `test.fixme` to `test`.
    - **Share** P1 (`SHARE-*`): invite, accept, decline, revoke, public list link, with a second user (register is rate-limited; seed via API).
    - **P2** as feasible: archive viewer, admin screenshots and search reindex, public list report, passkey and SSO edges, plus the other skipped `@p2` stubs.
    - Optional: a Playwright project that greps only `@smoke` for faster local runs.

---

## Far out

Revisit once the stages above are done.

- **Privacy-respecting usage statistics** (M). Know how a public instance is used without tracking people.
  - Aggregate counts only (active users, bookmarks saved, features used), no personal data, page content or URLs, and nothing per person.
  - Off by default on self-hosted instances. Operators can send the statistics to their own backend with their own key (an OpenTelemetry exporter, or a service such as Datadog), building on the OpenTelemetry tracing the API already exports.
  - The managed service uses the same hooks with its own backend.
  - Document exactly what is collected.
- **Client-held vault key** (L). True end-to-end encryption, where the server never sees the key. Makes vault search much harder.
- **Native mobile apps** (L). iOS and Android, separate from the responsive web app and PWA.
- **Desktop app** (M). A [Tauri](https://tauri.app/) wrapper around the web app for macOS, Windows and Linux.
- **Gamification** (M). Points and badges on top of likes and ratings.
- **Previews for sites behind bot protection** (M). Some sites, such as those behind Cloudflare, block the screenshot and metadata fetch. Investigate allowed options, such as falling back to the site's own OG data or a user-uploaded preview.
- **Welcome email** (S). A welcome email for new accounts, with Dean's avatar badge. Security emails stay plain.
- **Docs FAQ** (S). Frequently asked questions on the docs site.
- **Easter eggs** (S). Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), which needs React 19 support first).
