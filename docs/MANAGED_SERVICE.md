# Managed service on GCP

Status: exploration. Nothing here is scheduled yet. It expands the **Managed service** item under Far out in [ROADMAP.md](./ROADMAP.md).

The self-hosted release is stable. This document covers what it would take to run Linkdean as a hosted service (SaaS) with a marketing site, deployed on Google Cloud with Terraform, and which GCP services to use for each part. Linkdean stays self-hosted first: the same image runs both, and everything SaaS-only is off unless configured.

Linkdean is already multi-user, so the gap is not tenancy. It is that the app assumes **one process with a local disk**. Cloud Run runs several instances and gives each a temporary disk, which breaks four things.

## 1. Code changes before running more than one instance

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
- **Structured logs** (Now: item 3 in the roadmap) become required: Cloud Logging indexes JSON.
- **Library pagination** (Now: item 1) matters more at hosted scale, where libraries of any size share the same database.

## 2. Landing and marketing site

- A separate static site (Astro or similar) on the apex domain, with the app on `app.<domain>`.
- Hosted on Firebase Hosting, or a Cloud Storage bucket behind Cloud CDN, managed in the same Terraform.
- Content: what Linkdean is, features, pricing and plans, a "self-host it for free" link to [INSTALL.md](./INSTALL.md) and GitHub, and sign-up.
- Later, the documentation site from the roadmap lives beside it on `docs.<domain>`.

## 3. Using GCP services

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

## 4. Terraform

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

## 5. Phases

1. **Ready for several instances.** Cloud Storage for files, the worker split, shared rate limits, and a decision on vault keys. Ship library pagination and structured logs from the roadmap's Now stage.
2. **Infrastructure.** Terraform for a staging project: Cloud SQL, Cloud Run, Meilisearch on a VM, load balancer and DNS. Deploy it and load-test it.
3. **SaaS layer.** Open sign-up with a captcha, Stripe, plan limits, legal pages and the marketing site.
4. **Launch hardening.** Cloud Armor rules, alerting, a backup restore drill and a status page.

## Rough cost

A thin production setup (a small Cloud SQL instance, one warm Cloud Run instance, a small Meilisearch VM, the load balancer and a small Memorystore instance) is estimated at roughly £80 to £150 a month. This estimate has not been checked against current GCP pricing.

## Open questions

1. **Tenancy model.** One shared instance for every customer (recommended: it fits the plans in the organizations and teams design), or one instance per customer (stronger isolation, much higher cost and operations work)?
2. **Vault keys across instances.** Session affinity for now, or redesign key storage first? This choice sets most of the work in phase 1.
3. **Domain and brand.** Which domain hosts the marketing site and `app.`?
4. **Meilisearch.** Self-run on a VM, pay for Meilisearch Cloud, or invest in Postgres full-text search to drop it?
5. **Email provider.** Postmark, SendGrid, Amazon SES or another?
6. **Regions and data residency.** One region to start (which one), and do any customers need EU-only data?
7. **Pricing.** One paid personal plan or several, and what each plan's limits are.
