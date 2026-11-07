# Public beta on Nova (roadmap #10–#11 and #13)

Design agreed 2026-11-06. Covers the marketing site, documentation site, and free public beta on the homelab as one sequenced path. Membership pricing explore (#12) stays a separate follow-up.

## Problem

Product Now items through structured logs are done. The public beta still needs a face (`linkdean.app`), operator docs (`docs.linkdean.app`), and an isolated stack on Nova (`app.linkdean.app`) that does not share fate with personal services. Cloudflare already holds the zone; Tunnel and DNS records are not set up yet.

## Goals

- Ship a static **Astro** marketing site and a **Docusaurus** docs site in this monorepo.
- Run **app + site + docs** inside one **LXD VM** on Nova (`nova-bot`), not on the host Docker next to personal apps.
- Publish only through a **Cloudflare Tunnel** (no open router ports).
- Leave the stack deployable and hostname-routable before announcing; turn on SMTP, Turnstile, and the other go-live gates before inviting the public.

## Non-goals

- Stripe, plan gates, or published paid pricing (#12 explore can follow; charging stays later).
- Migrating or retiring a personal Linkdean instance (open question in MANAGED_SERVICE; out of this design).
- Multi-instance / GCP readiness.
- Seeding accounts, admins, or sample data on the public instance.
- Announcing the beta before the go-live checklist below is green.

## Decisions locked

| Topic | Choice |
| --- | --- |
| Marketing stack | Astro, static, in `apps/site` |
| Docs stack | Docusaurus, in `apps/docs`, built from existing `docs/` |
| Isolation | Dedicated LXD VM on Nova (LXD 5.21 LTS already present; empty guest list at survey time) |
| Delivery shape | Repo-first, then guest: compose and apps in git, then create/configure the VM |
| Cloudflare | Zone exists; add Tunnel + DNS as part of this work |
| Mail / Turnstile | Placeholders in env first; enable before announce, not before first Tunnel bring-up |
| Approach | One sequenced path covering roadmap #10, #11, and #13 |

## Architecture

```text
Internet
    │
 Cloudflare (DNS, CDN/cache for static, Tunnel)
    │
 cloudflared  ──►  Caddy (or nginx) in guest
                      ├─ linkdean.app        → Astro static
                      ├─ docs.linkdean.app   → Docusaurus static
                      └─ app.linkdean.app    → linkdean-api (serves API + web)
                           └─ postgres, meilisearch, optional chrome
```

- **Host (Nova):** LXD only for this project. No public Linkdean containers on the host Docker. Personal services stay on the host (or their own guests) and are not on the Tunnel routes for `linkdean.app`.
- **Guest (`linkdean-public`):** Docker Compose project with its own networks, volumes, and secrets. Chrome’s network must not reach the home LAN (firewall rule in addition to app-level private-address blocking).
- **Images:** App from GHCR release tags per INSTALL.md; site/docs are build artefacts (image or volume) produced from this repo’s CI or a guest-side build — prefer CI artefacts copied or baked into a tiny static image so the guest does not need a full Node toolchain at runtime.

## Repository layout

| Path | Role |
| --- | --- |
| `apps/site` | Astro marketing site |
| `apps/docs` | Docusaurus site (content from / mirrored to `docs/` as needed) |
| `deploy/public-beta/` | Compose file(s), Caddyfile, `cloudflared` sample config, `.env.example`, LXD bootstrap notes |
| `docs/MANAGED_SERVICE.md` | Status and checklist stay the operator source of truth; update as slices land |
| CI | Build/check `apps/site` and `apps/docs`; app images unchanged (GHCR) |

Do not put public-beta secrets in the repo. `deploy/public-beta/.env.example` lists required keys only.

## Marketing site (`apps/site`)

Static Astro site for the apex host.

**First viewport / pages (v1):** brand-forward hero (product name as hero signal), one headline, one short sentence, CTAs: **Sign up** → `https://app.linkdean.app/register` (or login), **Self-host** → install guide (docs or GitHub INSTALL). Feature highlights, real product screenshots (no invented metrics), links to Terms / Privacy / Acceptable Use on the app host (or copied static legal pages if we choose to avoid a hop — default: link to `app.linkdean.app` legal routes), GitHub, and abuse/contact `contact@linkdean.app` in the footer. Status link optional if a status page exists later.

**Explicitly not in v1:** pricing table (wait for #12), blog, account dashboard.

Visual direction follows the existing Linkdean brand assets under `brand/` and the web app; avoid generic AI-landing clichés (purple wash, fake stats, card soup). Hallmark / frontend-design skills apply when implementing UI.

## Documentation site (`apps/docs`)

Docusaurus at `docs.linkdean.app`, built from the existing markdown docs.

**v1 nav:** Self-hosting (INSTALL + env reference), Development setup, Features, FAQ/examples as available. API reference waits for OpenAPI / API platform. Prefer generating or copying from `docs/` so FEATURES/INSTALL do not drift; document the sync rule in the docs app README.

## Deploy package (`deploy/public-beta/`)

- Compose services: `api` (GHCR), `db`, `meili`, optional `chrome`, `caddy` (or `nginx`), `cloudflared`.
- Caddy (preferred for simple TLS-less local HTTP behind Tunnel): route by `Host` to static roots and to the API.
- `cloudflared`: one tunnel, three public hostnames → guest HTTP services.
- LXD notes: create Ubuntu VM, install Docker, clone or sync deploy dir, `lxd init` only if the host has never been initialized (ask before running), resource sizing (start modest: 2–4 vCPU, 4–8 GiB RAM, disk for Postgres + previews).
- Fixed Docker IPAM subnets if the host’s pools are exhausted (pattern from INSTALL / nova rollout notes).

## Cloudflare

- Create a Tunnel (token or credentials file living only on the guest).
- DNS (proxied): `linkdean.app`, `www` (redirect to apex), `app`, `docs` → Tunnel.
- Cache static hosts aggressively at Cloudflare; `app` mostly dynamic (sensible defaults, do not cache authenticated HTML).
- No router port forwarding for this stack.

## Go-live checklist (before announce)

From MANAGED_SERVICE, required before calling it a public beta:

1. Tunnel only; no open ports for these hostnames.
2. Chrome kept off the LAN (guest firewall).
3. Transactional SMTP + SPF, DKIM, DMARC on `linkdean.app`.
4. `REQUIRE_VERIFIED_EMAIL=true`, `BLOCK_DISPOSABLE_EMAILS=true`, Turnstile site + secret keys set; registration open for the beta.
5. Off-site backup of Postgres + avatars/previews volumes; restore tested once.
6. External uptime check + phone alert.
7. ISP terms allow serving a public site (operator confirmation).
8. `LOG_FORMAT=json` on the public API.

Until 3–4 are done, the app may be up for operator smoke tests; **do not announce** open sign-up.

## Delivery slices (reviewable PRs)

1. **Design + deploy skeleton** — **Done.**
2. **Guest + Tunnel + app** — **Done** (`app.linkdean.app` live).
3. **Marketing site** — **Done** ([linkdean.app](https://linkdean.app)).
4. **Docs site** — **Done** ([docs.linkdean.app](https://docs.linkdean.app)).
5. **Go-live hardening** — **Open.** SMTP/Turnstile/firewall/backup/uptime; remaining plan: [public-beta-homelab plan](../plans/2026-11-06-public-beta-homelab.md).

Roadmap bookkeeping: #10 and #11 are **Done**. Do not mark #13 Done until the go-live checklist is complete.

## Ops process on Nova

Follow `.claude/skills/linkdean-homelab-deploy/SKILL.md`: survey read-only, list every planned host change, wait for explicit go-ahead before editing the host or guest. Never seed the public instance. Prefer `novabot-local` on LAN/Tailscale; batch SSH when the tunnel is flaky. Host repos may contain WIP — do not commit there unless asked.

## Success criteria

- `https://app.linkdean.app` serves Linkdean from the LXD guest via Tunnel.
- `https://linkdean.app` serves the Astro marketing site.
- `https://docs.linkdean.app` serves Docusaurus built from repo docs.
- Personal services on Nova are untouched by this Compose project.
- Announce only after the go-live checklist is green.
