# Roadmap

Ordered milestones = build order. Each milestone lists concrete deliverables.

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

## Milestone 1 — Sharing infra (v2 core)

Foundation for collaboration. Personal dashboard does not *require* this technically; we still do sharing first so shared dashboards (M3) land on real ACL, not a second model.

**Deliverables**

- Share folders / lists with other users on the same instance
- Invite model with access levels: read-only / read-write
- Collaborative edit on shared collections (RW invitees)
- Schema next to `owner_user_id` (share / membership tables) before UI polish

**Out of this milestone**

- Public bookmark list URLs → later (backlog)
- Shared dashboards → Milestone 3

---

## Milestone 2 — Personal dashboard

Glance / Authentik-style home for a single user.

**Deliverables**

- User dashboard: sectioned grid of selected bookmarks
- Arrange / edit layout of frequently visited links
- Persisted per-user layout

---

## Milestone 3 — Shared dashboards

Depends on Milestone 1 (share/ACL) and builds on Milestone 2 (dashboard UX).

**Deliverables**

- Share a dashboard with invitees
- Creator controls invitee access: read-only / read-write
- Invitees see shared layout; RW can rearrange per ACL rules

---

## Milestone 4 — API keys

Programmatic access to a user’s own data. Independent of sharing; scheduled after dashboards.

**Deliverables**

- Users create / revoke API keys scoped to their own data
- Admin: enable/disable API keys for the whole platform
- Admin: enable/disable API keys per user
- Auth path accepts API key (Bearer) alongside session

---

## Milestone 5 — Archive & downloads (v3)

Heavy infra; keep after interactive product surfaces.

**Deliverables**

- Opt-in local cache of bookmarked pages
- Download / export: PDF, Markdown (reader mode), images, audio, video
- Separate storage volume + background job queue
- Deploy: archive worker + volume mount convention

---

## Milestone 6 — Browser extension

Lowest numbered priority.

**Deliverables**

- “Save current tab” to Linkdean
- Quick file-to-category from the extension

---

## Backlog (unscheduled)

Pick into a milestone when prioritized.

**Product**

- Public bookmark list URLs (after sharing infra is solid)
- Bulk edit / multi-select move, tag, delete
- Duplicate merge UI (API already lists duplicate URLs)
- Broken-link checker (rate-limited worker)
- Keyboard-first command palette (add, search, jump category)
- Reading list / “unread” + pin / favorites
- Full-text notes and optional highlights
- Favicon + OG image cache on disk (not hot-fetch forever)

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
- i18n-ready strings
- Mobile-friendly layout pass
- PWA install for “app-like” phone use
