# Managed service

Status: the near-term path is a **free public beta on the homelab**. A cheap VPS and a full GCP deployment are Far out in [roadmap.md](../product/roadmap.md). This document is the plan for all three.

Linkdean stays self-hosted first: the same image runs both, and everything SaaS-only (billing, plan gates) is off unless configured. The homelab beta needs no code changes for scaling: one server avoids every multi-instance problem listed under Moving to GCP.

## Public beta on a home server

Status: in progress. Public app, docs, and marketing are live on Nova via LXD + Cloudflare Tunnel (`app.linkdean.app`, [docs.linkdean.app](https://docs.linkdean.app), [linkdean.app](https://linkdean.app)). Docs source: [`apps/docs`](../../apps/docs) (Docusaurus → `deploy/public-beta/static/docs`). Marketing: [`apps/site`](../../apps/site) (Astro → `deploy/public-beta/static/site`; footer contact `contact@linkdean.app`). Next: finish go-live hardening (SMTP, Turnstile, uptime, restore drill) — do **not** announce until the checklist below is green. Membership pricing explore (#12) stays after that. Backups + Infisical: [BACKUP.md](../../deploy/public-beta/BACKUP.md). Deploy package: [`deploy/public-beta/`](../../deploy/public-beta/).

**Shape.** A stand-alone Linkdean stack, separate from any personal instance, serving the whole `linkdean.app` domain:

| Host                | What                        |
| ------------------- | --------------------------- |
| `linkdean.app`      | Marketing site (static)     |
| `app.linkdean.app`  | The app (API and web)       |
| `docs.linkdean.app` | Documentation site (static) |

**Capacity.** A modern mini PC (for example a Ryzen 7 with 16 threads, 27 GiB RAM and hundreds of GB free) runs a Linkdean stack in under 500 MiB at idle, with negligible CPU. The heaviest work is the screenshot burst after an import, which the worker throttles to two at a time. With 400–500 Mbps upload and Cloudflare caching static files, a home server should serve a free beta of dozens to low hundreds of active users. Those numbers are estimates, not load-tested.

### Isolate from the rest of the homelab

Yes — plan on it. A public `linkdean.app` is a different threat model from personal services behind Authentik or Tailscale.

| Risk | What goes wrong if the stack shares the host with other services |
| --- | --- |
| Lateral movement | A bug or RCE in the API, or a compromised dependency, reaches other Compose projects on the same Docker bridge or host |
| Chrome / SSRF | Headless Chrome fetches arbitrary URLs. App-level private-address blocking helps, but a bypass or misconfig that can hit the LAN (routers, NAS, admin UIs, other containers) is much worse if Chrome shares a network with them |
| Secret bleed | Shared `.env` files, bind-mounted host paths, or a common secrets volume leak personal-instance credentials into the public stack (or the reverse) |
| Resource contention | One user's import screenshot burst or Meilisearch rebuild starves Authentik, proxies, or a personal Linkdean on the same CPU, RAM or disk |
| Blast radius | `docker compose down -v`, a bad migration, or disk fill takes down everything on the host, not only the public beta |
| Abuse traffic | Scrapers and credential stuffing hit the public hostname; without isolation they also load the shared proxy, DNS and host firewall |

**Minimum isolation.** Its own Compose project, Docker networks, database, Meilisearch, Chrome, volumes and secrets. Nothing shared with other services on the host, and no host secrets mounted in.

**Better.** A separate VM (or equivalent) on the host, so a container escape still lands in an empty guest. Publish only through Cloudflare Tunnel; do not put the public stack on the same reverse-proxy routes as personal apps.

### What else has to be true before it is public

- **No open ports.** Publish through a Cloudflare Tunnel (`cloudflared`) instead of router port forwarding, so the home IP stays hidden and Cloudflare's DDoS protection and WAF sit in front. Cache the static sites at Cloudflare.
- **Chrome kept off the LAN.** Done in the stack: Chrome's `browser` network is `internal`, and its only way out is the API's egress proxy, which resolves every host itself and refuses private addresses. A WebSocket, service worker or DNS-rebinding trick that gets past the DevTools request check still has no route to the LAN. A host firewall rule is optional extra depth.
- **Email through a provider.** Mail from a home IP lands in spam. Send through a transactional provider's SMTP with SPF, DKIM and DMARC on `linkdean.app`.
- **Open registration with misuse controls.** Keep `registration_open` on for the free beta (invite-only is an admin escape hatch, not the default). Set `REQUIRE_VERIFIED_EMAIL=true` with SMTP, `BLOCK_DISPOSABLE_EMAILS=true`, and both Cloudflare Turnstile keys (`TURNSTILE_SITE_KEY` / `TURNSTILE_SECRET_KEY`).
- **Backups off-site.** Restic → dedicated Backblaze B2 bucket `linkdean-public` (Postgres dump + avatars/previews), scheduled on Nova three times a day, keeping the last 3 snapshots; guest env and restic keys from Infisical `/linkdean-public`. See [BACKUP.md](../../deploy/public-beta/BACKUP.md). A restore must be tested end to end before announce.
- **Monitoring.** An external uptime check (the host's own dashboard cannot report that the host is down) and alerts to a phone.
- **ISP terms.** Check the home broadband contract allows serving a public site.
- **Product and launch readiness.** [roadmap.md](../product/roadmap.md) Now items 1–11 are shipped in product; this deploy's go-live checklist (below) is what remains before announce. Membership pricing explore (#12) follows announce readiness.

### When to move off the home server

Any of: outages start to matter to users, upload bandwidth or CPU becomes the bottleneck, abuse traffic reaches the home connection, or the service starts charging. The next step is the same Compose stack on a single cheap VPS (Far out in the roadmap; the same [quick-start.md](../self-hosting/quick-start.md) applies). GCP, as described below, is for when one server is no longer enough.

## Moving to GCP

Far out in the [roadmap](../product/roadmap.md). One Compose host (homelab or a cheap VPS) stays the path until multi-instance is required. Before several API instances can run (for example Cloud Run), these assumptions have to change:

| Area | Today | Multi-instance need |
| --- | --- | --- |
| Unlocked vault keys | In API memory per session | Shared short-lived store or session affinity; longer term client-held key |
| Rate limits | In-process maps | Shared counters (Redis / Postgres) |
| Avatars and previews | Local disk volumes | Object storage abstraction |
| Background tasks | Spawned inside each API process | Separate worker / jobs so work is not duplicated |

Also needed for a paid hosted service: Stripe and plan gates (see [organizations and teams](../design/organizations-and-teams.md)), stronger abuse controls, legal/data-rights polish, and config-only feature flags so the self-hosted image stays ungated. Structured logs (`LOG_FORMAT=json`) and library pagination already ship.

Landing/docs/marketing already run as static sites for the homelab beta; on GCP they would move to Firebase Hosting or Cloud Storage + CDN. Prefer Terraform for staging then prod; rough thin-prod cost was estimated £80–£150/month (unchecked against current pricing).

## Open questions

1. **Personal instance.** Does the personal `linkdean.arsam.app` stay as it is once `linkdean.app` launches, or is it retired, with its account moved to the public instance?
2. **Tenancy model.** One shared instance for every customer (recommended: it fits the plans in the organizations and teams design), or one instance per customer (stronger isolation, much higher cost and operations work)?
3. **Vault keys across instances.** Session affinity for now, or redesign key storage first? This choice sets most of the work in GCP phase 1.
4. **Meilisearch.** Self-run on a VM, pay for Meilisearch Cloud, or invest in Postgres full-text search to drop it?
5. **Email provider.** Postmark, SendGrid, Amazon SES or another?
6. **Regions and data residency.** One region to start (which one), and do any customers need EU-only data?
7. **Pricing.** One paid personal plan or several, and what each plan's limits are.
