# Managed service

Status: the near-term path is a **free public beta on the homelab**. A cheap VPS and a full GCP deployment are Far out in [ROADMAP.md](./ROADMAP.md). This document is the plan for all three.

Linkdean stays self-hosted first: the same image runs both, and everything SaaS-only (billing, plan gates) is off unless configured. The homelab beta needs no code changes for scaling: one server avoids every multi-instance problem listed under Moving to GCP.

## Public beta on a home server

Status: in progress. Public app, docs, and marketing are live on Nova via LXD + Cloudflare Tunnel (`app.linkdean.app`, [docs.linkdean.app](https://docs.linkdean.app), [linkdean.app](https://linkdean.app)). Docs source: [`apps/docs`](../apps/docs) (Docusaurus → `deploy/public-beta/static/docs`). Marketing: [`apps/site`](../apps/site) (Astro → `deploy/public-beta/static/site`; footer contact `contact@linkdean.app`). Next: finish go-live hardening (SMTP, Turnstile, uptime, restore drill) — do **not** announce until the checklist below is green. Membership pricing explore (#12) stays after that. Design: [public-beta-homelab](./superpowers/specs/2026-11-06-public-beta-homelab-design.md); backups + Infisical: [restic-infisical](./superpowers/specs/2026-11-07-public-beta-restic-infisical-design.md) / [BACKUP.md](../deploy/public-beta/BACKUP.md); remaining ops plan: [public-beta-homelab plan](./superpowers/plans/2026-11-06-public-beta-homelab.md); deploy: [`deploy/public-beta/`](../deploy/public-beta/).

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
- **Backups off-site.** Restic → dedicated Backblaze B2 bucket `linkdean-public` (Postgres dump + avatars/previews), scheduled on Nova three times a day, keeping the last 3 snapshots; guest env and restic keys from Infisical `/linkdean-public`. See [BACKUP.md](../deploy/public-beta/BACKUP.md). A restore must be tested end to end before announce.
- **Monitoring.** An external uptime check (the host's own dashboard cannot report that the host is down) and alerts to a phone.
- **ISP terms.** Check the home broadband contract allows serving a public site.
- **Product and launch readiness.** [ROADMAP.md](./ROADMAP.md) Now items 1–11 are shipped in product; this deploy's go-live checklist (below) is what remains before announce. Membership pricing explore (#12) follows announce readiness.

### When to move off the home server

Any of: outages start to matter to users, upload bandwidth or CPU becomes the bottleneck, abuse traffic reaches the home connection, or the service starts charging. The next step is the same Compose stack on a single cheap VPS (Far out in the roadmap; the same [INSTALL.md](./INSTALL.md) applies). GCP, as described below, is for when one server is no longer enough.

## Moving to GCP

Linkdean is already multi-user, so the gap is not tenancy. It is that the app assumes **one process with a local disk**. Cloud Run runs several instances and gives each a temporary disk, which breaks four things.

### 1. Code changes before running more than one instance

| Area                 | Today                                                                                                                              | What breaks with several instances                                              | Fix                                                                                                                                                                                          |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unlocked vault keys  | Held in API memory per login session (`crates/api/src/vault_keys.rs`)                                                              | A request served by another instance sees the vault as locked                   | Short term: session affinity on Cloud Run and the load balancer. Proper fix: a short-lived, encrypted key store in Memorystore, or the client-held vault key (Far out)                       |
| Rate limits          | In-process `Mutex<HashMap>` (`crates/api/src/rate_limit.rs`)                                                                       | Each instance counts on its own, so limits become N times looser                | Shared counters in Memorystore (Redis) or Postgres                                                                                                                                           |
| Avatars and previews | Local directories (`AVATAR_DIR`, `PREVIEW_DIR`)                                                                                    | Cloud Run disks are temporary and not shared                                    | A storage abstraction (for example the `object_store` crate): local disk for self-hosted, Cloud Storage for the managed service                                                              |
| Background tasks     | Spawned inside the API process: maintenance, the mail sender, the preview backfill and the link checker (`crates/api/src/main.rs`) | Every instance runs every task; the preview backfill re-runs on each cold start | A separate worker service with one instance, or Cloud Run Jobs triggered by Cloud Scheduler. The email outbox and notification email already claim rows with `SKIP LOCKED`, so they are safe |

Also needed for a hosted service:

- **Billing and plans.** Stripe customers and subscriptions, and plan limits enforced in the API (bookmarks, storage, screenshots per month). The plans are set out in the [organizations and teams design](./superpowers/specs/2026-11-03-organizations-and-teams-design.md). Self-hosted instances stay ungated.
- **Abuse controls.** A captcha on sign-up, per-plan quotas on the screenshot and page-fetch services (they are both an SSRF surface and a cost), and Cloud Armor in front of everything.
- **Legal and data rights.** Terms of service, privacy policy, a data processing agreement, cookie consent, account deletion and full export. Import and export already exist.
- **Feature flags for the hosted service.** Billing and plan checks switch on through configuration only, so the self-hosted image never needs them.
- **Structured logs** are shipped (`LOG_FORMAT=json`); Cloud Logging indexes JSON. Keep `LOG_FORMAT=json` on hosted instances.
- **Library pagination** is shipped; it matters more at hosted scale, where libraries of any size share the same database.

### 2. Landing, docs, and marketing

- Homelab beta already ships Astro marketing on the apex, the app on `app.<domain>`, and Docusaurus on `docs.<domain>` (see Public beta above). On GCP, move those static sites to Firebase Hosting or a Cloud Storage bucket behind Cloud CDN in the same Terraform.
- Content already covered for beta: product pitch, features, self-host CTA, sign-up, legal links, abuse contact. Pricing and plans wait on roadmap #12.

### 3. Using GCP services

| Component            | GCP service                                                                                                               | Notes                                                                                                                                                                                                                          |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Postgres 16          | **Cloud SQL for PostgreSQL**                                                                                              | Private IP, point-in-time recovery, high availability in production. sqlx works unchanged. Connect through the Cloud SQL Auth Proxy, or private IP with IAM database authentication                                            |
| Avatars and previews | **Cloud Storage**                                                                                                         | Private bucket. Keep serving through the API, which checks access as it does today, or hand out short-lived signed URLs. Lifecycle rules remove orphaned objects                                                               |
| API and web app      | **Cloud Run**                                                                                                             | The existing image. Keep at least one instance warm until vault keys leave process memory                                                                                                                                      |
| Worker               | **Cloud Run** (one instance) or **Cloud Run Jobs** with **Cloud Scheduler**                                               | After the split described in section 1                                                                                                                                                                                         |
| Headless Chrome      | A separate **Cloud Run** service                                                                                          | Its own service account with no permissions, no route into the private network, and egress through Cloud NAT. Test whether Chrome's sandbox works on Cloud Run's second-generation runtime; do not fall back to `--no-sandbox` |
| Meilisearch          | **No managed GCP equivalent.** A Compute Engine VM with a persistent disk (cheapest), GKE Autopilot, or Meilisearch Cloud | Longer term, Postgres full-text search could remove this service                                                                                                                                                               |
| Secrets              | **Secret Manager**                                                                                                        | `SESSION_SECRET`, `MEILI_MASTER_KEY`, OIDC and SMTP credentials, Stripe keys                                                                                                                                                   |
| Shared state         | **Memorystore for Redis**                                                                                                 | Rate limits, and the vault key store if that route is chosen                                                                                                                                                                   |
| Ingress              | **Global external HTTPS load balancer**, **Cloud CDN**, **Cloud Armor**, Google-managed certificates, **Cloud DNS**       | CDN for the web app's static assets and the marketing site                                                                                                                                                                     |
| Images               | **Artifact Registry**                                                                                                     | Build into it, or use a remote repository that mirrors GHCR                                                                                                                                                                    |
| Email                | **No native GCP sending service.** Postmark, SendGrid or Amazon SES over SMTP                                             | The app already speaks SMTP                                                                                                                                                                                                    |
| Observability        | **Cloud Logging**, **Cloud Monitoring**, uptime checks, Error Reporting, **Managed Service for Prometheus**               | The existing `/metrics` endpoint feeds Managed Prometheus                                                                                                                                                                      |
| Backups              | Cloud SQL automated backups, plus a versioned Cloud Storage bucket                                                        | Meilisearch can be rebuilt from Postgres                                                                                                                                                                                       |

Not worth using yet: Identity Platform (Linkdean's own auth and OIDC already cover sign-in), and Pub/Sub or Cloud Tasks (the Postgres outbox is enough at this scale).

### 4. Terraform

```
infra/
  bootstrap/   state bucket (GCS backend), Workload Identity Federation for GitHub Actions
  modules/     network, sql, storage, run-service, meili-vm, lb, dns, secrets, iam, monitoring
  envs/
    staging/   one GCP project
    prod/      one GCP project
```

- **Network:** a VPC, Cloud Run direct VPC egress, Cloud NAT, and private services access for Cloud SQL and Memorystore.
- **IAM:** one service account per service with least privilege. The API gets Cloud SQL, its bucket and its secrets; Chrome gets nothing.
- **CI/CD:** GitHub Actions authenticates through Workload Identity Federation, with no stored keys. On a release tag: `terraform plan` and apply, run migrations as a Cloud Run Job, deploy the new image to staging, then promote to production.
- **Cost guards:** budget alerts and a monthly cap per project.

### 5. Phases

1. **Ready for several instances.** Cloud Storage for files, the worker split, shared rate limits, and a decision on vault keys. Library pagination and structured logs are already on main; run hosted with `LOG_FORMAT=json`.
2. **Infrastructure.** Terraform for a staging project: Cloud SQL, Cloud Run, Meilisearch on a VM, load balancer and DNS. Deploy it and load-test it.
3. **SaaS layer.** Stripe, plan limits, and any hosted-only gates. Captcha, legal pages, abuse reports, quotas, and the marketing/docs sites already ship for the free beta; reuse them.
4. **Launch hardening.** Cloud Armor rules, alerting, a backup restore drill and a status page.

### Rough cost

A thin production setup (a small Cloud SQL instance, one warm Cloud Run instance, a small Meilisearch VM, the load balancer and a small Memorystore instance) is estimated at roughly £80 to £150 a month. This estimate has not been checked against current GCP pricing.

## Open questions

1. **Personal instance.** Does the personal `linkdean.arsam.app` stay as it is once `linkdean.app` launches, or is it retired, with its account moved to the public instance?
2. **Tenancy model.** One shared instance for every customer (recommended: it fits the plans in the organizations and teams design), or one instance per customer (stronger isolation, much higher cost and operations work)?
3. **Vault keys across instances.** Session affinity for now, or redesign key storage first? This choice sets most of the work in GCP phase 1.
4. **Meilisearch.** Self-run on a VM, pay for Meilisearch Cloud, or invest in Postgres full-text search to drop it?
5. **Email provider.** Postmark, SendGrid, Amazon SES or another?
6. **Regions and data residency.** One region to start (which one), and do any customers need EU-only data?
7. **Pricing.** One paid personal plan or several, and what each plan's limits are.
