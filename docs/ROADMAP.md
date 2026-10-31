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
- Same-instance category sharing (read / write ACL, share UI, Meili visibility, RO library gates)

---

## Milestone 1: Sharing infra (v2 core) — done

Foundation for collaboration. Sharing landed before personal dashboards so shared dashboards (Milestone 3) reuse the same ACL model.

**Delivered**

- Share folders / lists with other users on the same instance
- Invite model with access levels: read-only / read-write
- Collaborative edit on shared collections (RW invitees)
- Schema next to `owner_user_id` (`category_shares`); library + explorer polish (RO gates, shared section)

**Out of this milestone**

- Public bookmark list URLs (backlog)
- Shared dashboards (Milestone 3)

---

## Milestone 2: Personal dashboard - done

Home surface for a single user (glance-style sectioned grid). `/` is the dashboard; library stays at `/library`.

**Delivered**

- User dashboard: sectioned grid of selected bookmarks
- Arrange / edit layout of frequently visited links
- Persisted per-user layout (`dashboards` + sections + items)
- Glance tiles + section reorder; `default_landing` `/` vs `/library` (Settings + post-login)
- Invitee share badges on cards / glance tiles / folder tiles
- App shell: **Dashboard** nav next to Library; logo click honors `default_landing`
- Owner-side share cue on bookmarks / folders you have shared out (dashboard cards with M3)
- Multi-dashboard: many boards, one default pin; page switcher + create / pin / delete; Settings default-dashboard select; rename via edit-layout title; delete pin falls back or recreates Home

**Before marking done**

- Quick UX QA (switcher, pin, empty board, RO shared folders in library still gated)
- User confirm → mark **done**, then Milestone 3

---

## Milestone 3: Shared dashboards

Depends on Milestone 1 (share/ACL) and builds on Milestone 2 (dashboard UX), including multi-dashboard.

**Deliverables**

- Share a dashboard with invitees
- Creator controls invitee access: read-only / read-write
- Invitees see shared layout; RW can rearrange per ACL rules
- Shared-dashboard visual cues consistent with folder/bookmark share badges
- Invitees can switch among dashboards shared with them (alongside own, once multi-dashboard ships)

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

Depends on Milestone 1 (share/ACL). Activity surfaces benefit from Milestone 3 (shared dashboards); notifications can start on shared folders/lists alone. Email delivery depends on Platform SMTP (backlog).

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
- Pending email invites for bookmark / list / dashboard shares (not public URLs): invite by address; if recipient has no account, they register and claim access
- Bulk edit / multi-select move, tag, delete
- Duplicate merge UI (API already lists duplicate URLs)
- Duplicate / fork a list (esp. shared → personal copy the invitee keeps)
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
- Admin users dashboard: list / manage accounts; never read private user data (bookmarks, categories, vault, etc.)
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
- SMTP / outbound email config (for invites + Milestone 7 notifications)
- Localization / i18n-ready strings (and eventual locale packs)
- Mobile-friendly layout pass
- Burger / collapse nav for mobile and very small viewports
- PWA install for app-like phone use

---

## Unplanned (far out)

Outside the numbered track. Revisit after planned milestones and backlog pressure ease.

- Native mobile apps (iOS / Android), separate from responsive web + PWA
