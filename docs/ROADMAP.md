# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

---

## Backlog 

**Product**

- Public bookmark list URLs (after sharing infra is solid)
- Pending email invites for bookmark / list / dashboard shares (not public URLs): invite by address; if recipient has no account, they register and claim access
- Broken-link checker (rate-limited worker)
- Full-text notes and optional highlights

**Vault & privacy**

- Client-held vault key option (true E2E; harder search tradeoff)
- Per-category vault passphrases (today: one vault per user)

**Auth & multi-user**

- Passkeys / WebAuthn
- Hardened OIDC (PKCE + discovery) for all providers, not just Authentik-shaped URLs

**Search & data**

- Personal activity history: per-item create/edit/delete log + filterable History drawer (bookmark / category / dashboard). Distinct from shared activity (notifications / comments)

**Platform**

- SMTP / outbound email config (for invites + notification delivery)
- Localization / i18n-ready strings (and eventual locale packs)
- Mobile-friendly layout pass

---

## Final UI/UX pass

before releasing the app to the public, we need to do a final pass through the UI/UX to ensure it is polished and ready for release. reimagine the UI with a strong UI/UX skill. and make sure app is responsive and usable on mobile divices.

---

## Unplanned (far out)

Outside the numbered track. Revisit after planned milestones and backlog pressure ease.

0. Improve testing coverage and add more tests for the app.

1. Native mobile apps (iOS / Android), separate from responsive web + PWA

2. Archive & downloads (v3)

Heavy infra; scheduled after interactive product surfaces.

**Deliverables**

- Opt-in local cache of bookmarked pages
- Download / export: PDF, Markdown (reader mode), images, audio, video
- Separate storage volume + background job queue
- Deploy: archive worker + volume mount convention

3. Improved gamification (points / badges on top of likes & ratings)

4. Browser extension

Lowest numbered priority among the planned interactive surfaces.

**Deliverables**

- Save current tab to Linkdean
- Quick file-to-category from the extension