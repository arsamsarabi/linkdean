# Roadmap

What is still planned, in build order (highest priority first). Shipped capabilities live in [features.md](./features.md). Scratch ideas: [ideas.md](./ideas.md).

Effort is a rough guess: **S** is days, **M** is one to two weeks, **L** is several weeks. Horizon tags: **`near`** (next to pick up), **`mid`**, **`parked`**.

Each feature lists delivery phases (or a single milestone). Ship a phase, then delete or rewrite it; when the whole feature ships, remove it, renumber the list, and move wording to features + changelog. The public [roadmap page](https://docs.linkdean.app/docs/roadmap) is a curated mirror: same order, horizon + next phase title only — no ops section, and no internal-only items (see skill notes).

Open defects are not listed here. They are `Known issue:` comments at the code they concern: `grep -rn "Known issue:" crates apps/web/src deploy`.

---

## Features

1. **Agent Skills, next steps** · `near` · (S–M). Folders and tags already download as Agent Skills; an API key can fetch them (see features). Extend the agent surface without giving up the privacy stance.
    - **Phase A — MCP:** Read-only MCP server (search and read bookmarks), as well as or instead of generated files.
    - **Phase B — Archive text:** Include archived reader-mode text so the agent can read a page without fetching it when a copy exists.
    - **Phase C — Saved searches:** Skills from a saved library search, once saved searches exist.

2. **Legal review** · `near` · (S). A lawyer reviews the Terms and Privacy Policy before anything costs money; bump versions when the reviewed text ships.

3. **Plans and pricing** · `near` · (M). Decide what stays free on the managed service, what a paid plan adds, and the limits of each. Self-hosting stays free for non-commercial use.
    - **Phase A — Define tiers:** Free vs paid limits, including archive storage.
    - **Phase B — Enforce quotas:** Wire plan limits into platform / account quotas.
    - **Phase C — Checkout:** Billing / payment integration (may trail B).

4. **Organizations and teams** · `mid` · (L). Organizations own shared content in team spaces, with roles, restricted folders, email invites and a basic audit log. Personal libraries stay private. Requirements: [organizations and teams design](../design/organizations-and-teams.md).
    - **Phase A — v1:** Orgs, team spaces, owner/admin/member/guest, restricted folders, email invites, lifecycle rules, basic audit log.
    - **Phase B — Invite links:** Role, expiry, use limit; off by default.
    - **Phase C — Verified domains:** DNS TXT proof, then auto-join for matching addresses.
    - **Phase D — SSO / SCIM:** SAML or per-organization OIDC, plus SCIM provisioning.
    - **Phase E — Team vaults:** Shared keys, rotation when members leave, recovery.
    - **Phase F — Audit and roles:** Richer audit (retention, SIEM/webhook, hash chain); custom roles and Billing manager when needed.

5. **Localization** · `mid` · (M). Ship the product in more than English.
    - **Phase A — i18n-ready:** Extract and wrap UI strings.
    - **Phase B — Locale packs:** First translated locale(s).

6. **Archive follow-ups** · `parked` · (L). Core archive already ships (see [features](./features.md#archive) and the [archive design](../design/archive.md)).
    - **Phase A — Out-of-band media:** Fetch plain `<video>` / `<audio>` URLs that never landed in the Chrome snapshot (not streams), under item and account caps; fail soft and leave the live link.
    - **Phase B — Version history:** Keep more than one copy per format.
    - **Phase C — Per-tier limits:** Hosted per-plan archive storage (needs item 3); today only the platform-wide archive quota applies.

7. **AI assistance** · `parked` · (M–L). Karakeep, Linkwarden and Raindrop.io (Pro) offer AI tagging or summaries; Linkdean has none. Add it without giving up the privacy stance.
    - **Phase A — Opt-in provider:** Off by default; BYOK (OpenAI-compatible or Anthropic) or local Ollama on self-host; vault contents never leave for a model; settings show what is sent.
    - **Phase B — Tag and folder suggestions:** When saving and in **File one by one** (extends same-site suggestions); nothing applied without a click unless auto-tagging is on.
    - **Phase C — Summaries:** Short summary per bookmark from archived page text when present.
    - **Phase D — Semantic search:** Meilisearch hybrid search with embeddings ("that article about…").
    - **Phase E — Related / near-duplicates:** Hints for related and near-duplicate bookmarks.
    - **Phase F — Dashboards:** Suggest sections and sources from what the person saves and opens.

8. **Privacy-respecting usage statistics** · `parked` · (M). Know how a public instance is used without tracking people.
    - Aggregate counts only (active users, bookmarks saved, features used); no personal data, page content or URLs, nothing per person.
    - Off by default on self-hosted instances; operators can export to their own backend (OpenTelemetry / Datadog-style), building on tracing the API already exports.
    - Managed service uses the same hooks; document exactly what is collected.

9. **Previews for sites behind bot protection** · `parked` · (M). Some sites (e.g. behind Cloudflare) block screenshot and metadata fetch. Investigate allowed options such as the site's own OG data or a user-uploaded preview.

10. **Welcome email** · `parked` · (S). Welcome email for new accounts, with Dean's avatar badge. Security emails stay plain.

11. **Docs FAQ** · `parked` · (S). Frequently asked questions on the docs site.

12. **Desktop app** · `parked` · (M). A [Tauri](https://tauri.app/) wrapper around the web app for macOS, Windows and Linux.

13. **Native mobile apps** · `parked` · (L). iOS and Android, separate from the responsive web app and PWA.

14. **Client-held vault key** · `parked` · (L). True end-to-end encryption where the server never sees the key. Makes vault search much harder.

15. **Gamification** · `parked` · (M). Points and badges on top of likes and ratings.

16. **Easter eggs** · `parked` · (S). Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), which needs React 19 support first). Internal only — never list on the public docs roadmap.

---

## Ops / hygiene

Not ranked against product features. Raise coverage as we go; every fix and feature already ships with tests. Playwright `@smoke` and most `@p1` already run on the throwaway stack in PR CI; remaining journey stubs stay in `apps/web/e2e/`:

- Enable **LIB-02** (invalid URL on blur blocks Add). The drawer now validates; confirm smoke, then flip `test.fixme` to `test`.
- **Share** P1 (`SHARE-*`): invite, accept, decline, revoke, public list link, with a second user (register is rate-limited; seed via API).
- **P2** as feasible: archive viewer, admin screenshots and search reindex, public list report, passkey and SSO edges, plus the other skipped `@p2` stubs.
- Optional: a Playwright project that greps only `@smoke` for faster local runs.
