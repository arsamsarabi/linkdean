# Roadmap

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

## v2 — sharing & collaboration

- Share lists / categories with other users
- Collaborative edit on shared collections
- Public bookmark list URLs
- ACL / invite model (builds on existing `owner_user_id`)

## v3 — archive & downloads

- Opt-in local cache of bookmarked pages
- Download / export content: PDF, Markdown (reader mode), images, audio, video
- Separate storage volume + background job queue (do not block v1 schema)

---

## Improvements & enhancement ideas

**Product**

- Browser extension: “save current tab” + quick file-to-category
- Bulk edit / multi-select move, tag, delete
- Duplicate merge UI (API already lists duplicate URLs)
- Broken-link checker (rate-limited worker)
- Keyboard-first command palette (add, search, jump category)
- Reading list / “unread” + pin / favorites
- Full-text notes and optional highlights
- Favicon + OG image cache on disk (not hot-fetch forever)

**Vault & privacy**

- Client-held vault key option (true E2E; harder search tradeoff)
- Auto-lock after idle timeout
- Per-category vault passphrases (today: one vault per user)

**Auth & multi-user**

- Admin UI: invite users, disable open registration, promote admin
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

**v2/v3 prep**

- Share tables sketched beside `owner_user_id` before building UI
- Archive worker crate stub + volume mount convention early so deploy stays stable
