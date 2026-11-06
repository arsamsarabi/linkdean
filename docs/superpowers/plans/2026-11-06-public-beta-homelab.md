# Public beta on Nova Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver marketing site, docs site, and isolated public Linkdean on Nova via LXD + Cloudflare Tunnel, per `docs/superpowers/specs/2026-11-06-public-beta-homelab-design.md`.

**Architecture:** Repo-first: `deploy/public-beta/` Compose + Caddy + cloudflared samples, then `apps/site` (Astro) and `apps/docs` (Docusaurus). On Nova, one LXD VM runs that stack; Cloudflare Tunnel publishes three hostnames. Mail/Turnstile before announce.

**Tech Stack:** LXD 5.21, Docker Compose, Caddy, cloudflared, Astro, Docusaurus, GHCR `linkdean` image.

**Spec:** `docs/superpowers/specs/2026-11-06-public-beta-homelab-design.md`

---

## File map

| Path | Responsibility |
| --- | --- |
| `deploy/public-beta/docker-compose.yml` | Public stack: api, db, meili, caddy, cloudflared; chrome optional |
| `deploy/public-beta/Caddyfile` | Host-based routing for apex / app / docs |
| `deploy/public-beta/cloudflared.yml.example` | Tunnel ingress sample |
| `deploy/public-beta/.env.example` | Required secrets and public URLs |
| `deploy/public-beta/README.md` | Operator steps: LXD guest, Tunnel, go-live checklist |
| `apps/site/` | Astro marketing (slice 3) |
| `apps/docs/` | Docusaurus (slice 4) |
| `docs/MANAGED_SERVICE.md` | Status + pointers |

---

### Slice 1 — Deploy skeleton + docs pointer

### Task 1: `deploy/public-beta` Compose and Caddy

**Files:**
- Create: `deploy/public-beta/docker-compose.yml`
- Create: `deploy/public-beta/Caddyfile`
- Create: `deploy/public-beta/cloudflared.yml.example`
- Create: `deploy/public-beta/.env.example`
- Create: `deploy/public-beta/README.md`
- Create: `deploy/public-beta/static/.gitkeep` (placeholder roots until site/docs builds exist)
- Create: `deploy/public-beta/static/site/.gitkeep`
- Create: `deploy/public-beta/static/docs/.gitkeep`

- [ ] **Step 1:** Add Compose project `name: linkdean-public` based on INSTALL compose: `db`, `meilisearch`, `api` (GHCR, no published host ports — only internal), optional `chrome` from INSTALL pattern, `caddy` mounting `Caddyfile` + `static/`, `cloudflared` with config + credentials volume. Use fixed IPAM subnets documented as “pick unused on host/guest”. API `APP_URL=https://app.linkdean.app`, `LOG_FORMAT=json`. Bind nothing to `0.0.0.0` on the guest for public ports; Caddy listens on `:80` for the Tunnel to hit.

- [ ] **Step 2:** Caddyfile routes:
  - `linkdean.app`, `www.linkdean.app` → `file_server` root `static/site` (www redirect to apex)
  - `docs.linkdean.app` → `static/docs`
  - `app.linkdean.app` → `reverse_proxy api:8080`

- [ ] **Step 3:** `.env.example` with POSTGRES_PASSWORD, MEILI_MASTER_KEY, SESSION_SECRET, APP_URL, LINKDEAN_VERSION, LOG_FORMAT, placeholders for SMTP/Turnstile/TUNNEL token notes.

- [ ] **Step 4:** README: LXD VM create sketch (`lxc launch ubuntu:24.04 --vm linkdean-public`), Docker install, copy this dir, Cloudflare Tunnel create + DNS, `docker compose up`, go-live checklist link to MANAGED_SERVICE / spec. Explicit: ask before `lxd init`; no account seeding.

- [ ] **Step 5:** Update `docs/MANAGED_SERVICE.md` status: design + deploy package in repo; next is guest bring-up. Point at `deploy/public-beta/` and the design spec.

- [ ] **Step 6:** Commit and push on the design branch / PR.

---

### Slice 2 — Guest + Tunnel + app (manual + compose)

Requires user go-ahead on Nova (homelab skill). Plan tasks after slice 1 lands:

- Survey Nova LXD (`lxc list` only if already initialized; do not re-trigger installer casually).
- List every host/guest change; wait for explicit go.
- Create VM, install Docker, deploy `deploy/public-beta`, wire Tunnel DNS for `app` first (site/docs can 404 static until slices 3–4).
- Smoke: health, register page loads; do not announce.

### Slice 3 — `apps/site` Astro

- Scaffold Astro in `apps/site`, brand hero, CTAs, features, legal/GitHub links.
- CI build; artefact into `deploy/public-beta/static/site` or static image.
- Apex DNS + Tunnel route already in Caddyfile.

### Slice 4 — `apps/docs` Docusaurus

- Scaffold from existing docs; sync rule in README.
- CI build → `static/docs`.

### Slice 5 — Go-live hardening

- SMTP + DNS auth, Turnstile, firewall, backup restore, uptime.
- Close ROADMAP #10/#11/#13 and FEATURES when checklist green.

---

### Spec coverage (slice 1)

| Spec item | Task |
| --- | --- |
| `deploy/public-beta/` | Task 1 |
| Caddy host routing | Task 1 |
| cloudflared sample | Task 1 |
| LXD notes | Task 1 README |
| MANAGED_SERVICE pointer | Task 1 Step 5 |
| Astro / Docusaurus / live guest | Slices 2–4 |
