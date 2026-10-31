# Roadmap

Ordered milestones define build order. Each milestone lists concrete deliverables.

## Shipped (v1)

- Nested categories + tags
- Meilisearch search (non-vault)
- Import / export (Netscape, JSON, Pocket, Raindrop)
- Multi-device via server account
- Hidden categories
- Password-protected vault categories
- Local auth + optional OIDC
- First-user onboarding (admin)
- Docker Compose self-host path
- Library grid + explorer views (preference persisted)
- Admin: open registration toggle
- Vault auto-lock setting (UI)

---

## Milestone 1: Sharing infra (v2 core)

Foundation for collaboration. Sharing lands before personal dashboards so shared dashboards (Milestone 3) reuse the same ACL model rather than a parallel one.

**Deliverables**

- Share folders / lists with other users on the same instance
- Invite model with access levels: read-only / read-write
- Collaborative edit on shared collections (RW invitees)
- Schema next to `owner_user_id` (share / membership tables) before UI polish

**Out of this milestone**

- Public bookmark list URLs (backlog)
- Shared dashboards (Milestone 3)

---

## Milestone 2: Personal dashboard

Home surface for a single user (glance-style sectioned grid).

**Deliverables**

- User dashboard: sectioned grid of selected bookmarks
- Arrange / edit layout of frequently visited links
- Persisted per-user layout

---

## Milestone 3: Shared dashboards

Depends on Milestone 1 (share/ACL) and builds on Milestone 2 (dashboard UX).

**Deliverables**

- Share a dashboard with invitees
- Creator controls invitee access: read-only / read-write
- Invitees see shared layout; RW can rearrange per ACL rules

---

## Milestone 4: API keys

Programmatic access to a user's own data. Independent of sharing; scheduled after dashboards.

**Deliverables**

- Users create / revoke API keys scoped to their own data
- Admin: enable/disable API keys for the whole platform
- Admin: enable/disable API keys per user
- Auth path accepts API key (Bearer) alongside session

---

## Milestone 5: Archive & downloads (v3)

Heavy infra; scheduled after interactive product surfaces.

**Deliverables**

- Opt-in local cache of bookmarked pages
- Download / export: PDF, Markdown (reader mode), images, audio, video
- Separate storage volume + background job queue
- Deploy: archive worker + volume mount convention

---

## Milestone 6: Browser extension

Lowest numbered priority among the planned interactive surfaces.

**Deliverables**

- Save current tab to Linkdean
- Quick file-to-category from the extension

---

## Milestone 7: Shared activity & social

Depends on Milestone 1 (share/ACL). Activity surfaces benefit from Milestone 3 (shared dashboards); notifications can start on shared folders/lists alone.

Internal build order: notifications, discussion, lightweight reactions; gamification last (or backlog if scope expands).

**Deliverables**

- Notifications for shared activity (invites, edits, membership changes)
- Comments on shared bookmarks / lists / dashboards
- Likes on shared bookmarks / lists / dashboards
- Ratings on shared bookmarks / lists / dashboards
- Light gamification (optional; defer to backlog if it threatens the rest)

---

## Backlog (unscheduled)

Items may be promoted into a milestone when prioritized.

**Product**

- Public bookmark list URLs (after sharing infra is solid)
- Bulk edit / multi-select move, tag, delete
- Duplicate merge UI (API already lists duplicate URLs)
- Broken-link checker (rate-limited worker)
- Keyboard-first command palette (add, search, jump category)
- Reading list / unread + pin / favorites
- Full-text notes and optional highlights
- Favicon + OG image cache on disk (not hot-fetch forever)
- Reimagine category management UX
- Category icons

**Vault & privacy**

- Client-held vault key option (true E2E; harder search tradeoff)
- Per-category vault passphrases (today: one vault per user)

**Auth & multi-user**

- Admin UI: invite users, promote admin (registration gate already shipped)
- Passkeys / WebAuthn
- Hardened OIDC (PKCE + discovery) for all providers, not just Authentik-shaped URLs

**Search & data**

- Wait/retry UX when Meili is briefly behind writes
- Reindex button for admins
- Search filters: tag, category, date, vault-only
- Soft-delete + trash / undo

**Platform**

- Official `docker compose` one-liner docs + healthchecks in README badge style
- Prometheus metrics / basic ops endpoint
- Backup recipe (pg_dump + meili dump) in docs
- Localization / i18n-ready strings (and eventual locale packs)
- Mobile-friendly layout pass
- PWA install for app-like phone use

---

## Unplanned (far out)

Outside the numbered track. Revisit after planned milestones and backlog pressure ease.

- Native mobile apps (iOS / Android), separate from responsive web + PWA
