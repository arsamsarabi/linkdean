# Linkdean: homelab to Google Cloud migration

Status: planned, not started. Last updated 2026-11-15.

This folder moves to the private `linkdean-infra` repo with the rest of `docs/ops/` in Phase 1. Your hand-done steps are in [manual-setup.md](manual-setup.md), in the order the phases need them.

## Summary

The public beta moves from the Nova LXD guest to Google Cloud in London:

- **API:** Cloud Run, one instance that scales to zero.
- **Postgres:** the smallest Cloud SQL tier.
- **Search, screenshots and tunnel:** one small VM.

Email moves from iCloud Mail to Amazon SES in London. The docs and marketing sites move to Cloudflare Pages. Everything is defined in OpenTofu.

Before any of that, the public `linkdean` repo is cut down to the app itself. The sites, the deploy code and the managed-service notes move to two private repos.

The estimated cost is $45 to $80 a month. The move needs one maintenance window of about 30 minutes.

### Order of work

1. **Phase 0, accounts and tools (yours).** Do these any time before Phase 3. They can run alongside Phase 1, except the Cloudflare Pages token, which Phase 1 needs.
2. **Phase 1, repo split.** This comes first, before any Google Cloud work.
3. **Phase 2, product changes** in `linkdean`, shipped as 0.0.12 to the homelab.
4. **Phases 3 to 6:** the infrastructure and a full rehearsal with real data, while the homelab keeps serving users.
5. **Phases 7 to 9:** the legal update, cutover day, then retiring the homelab beta.

## Decisions

| Area | Decision |
| --- | --- |
| Region | London, `europe-west2`. Data stays in the UK |
| API | Cloud Run: 1 vCPU and 1 GiB, at most 1 instance, scales to zero. CPU is kept on while the instance is up. A timer wakes it hourly for housekeeping |
| Postgres | Cloud SQL for PostgreSQL 16, `db-f1-micro`, 10 GB SSD, private IP only. Daily backups kept 7 days, plus 7 days of point-in-time recovery |
| Search, screenshots, tunnel | One `e2-small` VM (2 GB) running Meilisearch, headless Chrome, the egress proxy and `cloudflared`. Cloud NAT gives it outbound traffic |
| Avatars and previews | A Cloud Storage bucket mounted into Cloud Run as a folder |
| How traffic reaches the API | A new Cloudflare tunnel, `linkdean-gcp`, runs from the VM. Cloud Run ingress is internal only |
| Email | Amazon SES in `eu-west-2` (London), on the Essentials plan new accounts start on |
| Docs and marketing sites | Cloudflare Pages. Files are uploaded from `linkdean-sites` |
| Infrastructure as code | OpenTofu. State lives in a versioned Cloud Storage bucket, encrypted with a Cloud KMS key |
| Repos | `linkdean` (public, the app only), `linkdean-infra` (private), `linkdean-sites` (private) |
| Deploys | Started by hand from `linkdean-infra` with `gh workflow run`. The public repo has no deploy step |
| Page view statistics | Dropped. The GoatCounter beacon and `stats.linkdean.app` go |
| Monitoring | Cloud Monitoring uptime checks on the app, docs and site. Alerts go to email and the Google Cloud phone app |
| Environments | Production only. Staging comes later, as a second OpenTofu variables file |
| Homelab | The personal instance at `linkdean.arsam.app` stays. The public-beta guest, its tunnel and its B2 backups are retired 14 days after cutover |

### Why OpenTofu

- **Staging later is cheap.** It is one more variables file, not another round of console clicks.
- **Every change is a reviewed diff.** `tofu plan` shows what will change before anything does.
- **Cloudflare and AWS live in the same place as Google.** DNS, the tunnel, the Pages projects and SES's DNS records sit next to the Google resources, so cutover is a one-line change.
- **State is encrypted on the client side.** OpenTofu supports this and Terraform does not, so a copied state file is unreadable without the KMS key.
- **Rebuilding after a mistake is a command,** not a memory exercise.

Some things stay out of OpenTofu on purpose:

- **Secret values.** OpenTofu creates empty Secret Manager secrets, and you add the values with `gcloud`. The same goes for the database user's password, the tunnel token and the SES SMTP password.
- **Which image runs.** The deploy workflow sets it on each release, and OpenTofu ignores that field.
- **The one-off data move.** That is a runbook (Phase 8), not infrastructure.

### Why a tunnel instead of a custom domain on Cloud Run

Cloud Run's own domain mapping is not offered in `europe-west2`, and it is still in preview. Google's load balancer, the recommended alternative, costs about $18 a month on its own.

Firebase Hosting forwards only a cookie named `__session` to Cloud Run, which would break `linkdean_session`.

## Monthly cost

These are rough London prices, worked out from published tier 1 rates and third-party trackers. You check them in the pricing calculator in Phase 0.

| Item | Est. $/month |
| --- | --- |
| Cloud Run | 10–20 |
| Cloud SQL `db-f1-micro` | 16–20 |
| `e2-small` VM, disks, Cloud NAT | 16–24 |
| Cloud Storage | under 1 |
| Artifact Registry, Secret Manager, Cloud KMS, Logging (30 days), Monitoring | 0–3 |
| Amazon SES: $0.16 per 1,000 emails on Essentials, no monthly fee | under 1 |
| Cloudflare Pages | 0 |
| **Total** | **about 45–80** (£35–65) |

- **Always-on API.** Keeping one instance running instead of scaling to zero adds about $45 to $55 a month.
- **Trial credit.** A Google account that has never used the free trial gets $300 of credit for 90 days. A new AWS account gets $100 of credit.
- **Monitoring is free at this size.** Uptime checks get 1 million runs a month free per project. Three checks every minute from three regions is about 390,000 runs. Alerts on uptime checks are exempt from the alerting charges Google plans to start no sooner than 1 September 2027.

## Repo split

The goal is that `linkdean` holds only what someone needs to build, run or self-host the app. It should hold nothing about how linkdean.app is hosted, deployed or marketed.

| Repo | Holds | Visibility |
| --- | --- | --- |
| `linkdean` | `crates/`, `apps/web`, `apps/extension`, migrations, `deploy/docker/` for self-hosters, the markdown under `docs/` (self-hosting, guides, product, design, CHANGELOG), the OpenAPI spec, README screenshots, and the logo kit (`brand/final/`, `brand/exports/`, `brand/GUIDELINES.md`) | Public |
| `linkdean-sites` | `apps/site` (Astro), `apps/docs` (Docusaurus, including its own pages such as the intro, getting started, beta, compare and the public roadmap), the brand working files (`brand/board`, `concepts`, `graphics`, `renders`, `node_render`, `HANDOVER.md`), and the Pages deploy and CI workflows | Private |
| `linkdean-infra` | OpenTofu, the VM files, the deploy workflows (homelab now, Google Cloud later), `deploy/public-beta/`, `docs/ops/` including this plan, the go-live checklist and the roadmap items about running linkdean.app | Private |

### How the docs site still follows the code

The docs site's sync step checks out `linkdean` at the newest release tag. The repo is public, so this needs no token. It copies the same markdown and OpenAPI spec as today.

A docs change for a feature still lands in the same `linkdean` pull request as the feature. It goes live when the docs site is next deployed after the release. Two files move so the app repo no longer points into `apps/docs`:

- The OpenAPI spec moves from `apps/docs/static/openapi.json` to `docs/api/openapi.json`. The `committed_spec_is_current` test in `crates/api/src/routes/mod.rs` writes it there.
- The README screenshots move from `apps/docs/static/img/screenshots/` to `docs/media/screenshots/`. The docs site copies them from there.

### No deploy hook in the public repo

The first draft kept one step in `linkdean`'s release workflow, using a token to tell `linkdean-infra` to deploy. That step is dropped, for two reasons:

- It is deploy code in the public repo.
- A token there that can start `linkdean-infra` workflows makes a compromised release job more dangerous.

Instead, after a release:

1. **Tag in `linkdean`.** CI runs, the image goes to GHCR, the extension job runs if its version changed, and the GitHub release is created.
2. **Deploy.** `gh workflow run deploy.yml -R arsamsarabi/linkdean-infra -f version=X.Y.Z`. The target is the homelab until cutover and Google Cloud after it. I run this as part of each release.
3. **Docs.** `gh workflow run deploy-docs.yml -R arsamsarabi/linkdean-sites -f version=X.Y.Z`. A daily scheduled check also rebuilds the docs when a newer release tag exists.

### History

The moved paths keep their history: they are copied with `git filter-repo` into the new repos. Old commits that touched them stay in the public history. Nothing secret was committed there, so this split is about what the repo looks like from now on.

## Phase 0: accounts and tools

All yours; see [manual-setup.md](manual-setup.md). Nothing here costs money yet.

- [ ] Cloudflare token for Pages deploys (needed in Phase 1).
- [ ] Install `gcloud`, `opentofu` and `awscli`.
- [ ] Google Cloud project `linkdean-prod`, with billing and a budget.
- [ ] AWS account on the Paid plan, with root MFA, a budget, and an IAM Identity Center admin user.
- [ ] Cloudflare token for OpenTofu, stored in Infisical.
- [ ] Price the setup in the Google Cloud pricing calculator.

## Phase 1: repo split

I do the work and open the pull requests. You review, add the Pages custom domains, and approve the one host change.

### 1a. New repos

- [ ] **Claude:** create the private repos `arsamsarabi/linkdean-infra` and `arsamsarabi/linkdean-sites`. You approved this on 2026-11-15.
- [ ] **Claude:** copy the paths in the table above into each repo with `git filter-repo` from a fresh clone, then push `main`.
- [ ] **Claude, `linkdean-sites`:**
  - Make the docs sync step read from a `linkdean` checkout (`LINKDEAN_DIR`) instead of `../../docs`.
  - Add a CI workflow that builds both sites on pull requests.
  - Add `deploy-site.yml` and `deploy-docs.yml`, which build and upload with `wrangler pages deploy`, plus the daily docs check.
  - Both sites gain a `release.txt`, so a check can confirm which build is live.
- [ ] **Claude:** move the local Claude skills with what they cover. These are untracked files under `.claude/skills/`. `linkdean-marketing-site` and `static-docs-deploy-verification` go to `linkdean-sites`, and `linkdean-deployment` and `linkdean-backup-restore-drill` go to `linkdean-infra`.

### 1b. Sites on Cloudflare Pages

This is the one hosting change in Phase 1. Without it, `linkdean-sites` would need a throwaway path back to Nova for a few weeks.

- [ ] **Claude:** create two Pages projects with wrangler, `linkdean-site` and `linkdean-docs`. Then deploy each to its `*.pages.dev` address and compare it page by page with the live site.
- [ ] **Claude:** carry over what the guest's Caddyfile does:
  - **www and apex:** both serve the marketing site, so both domains go on the same project.
  - **Marketing site:** falls back to its index page.
  - **Docs:** serve a pre-rendered page per path and their own 404 page.

  Pages does the last two by default. I check each after the switch.
- [ ] **You:** add the custom domains (manual-setup.md, step 2). Note each old DNS target first: pointing the record back at the homelab tunnel is the rollback.
- [ ] **Check:** `linkdean.app`, `www.linkdean.app` and `docs.linkdean.app` in a private window, a missing page on each, and each `release.txt`.

### 1c. Homelab deploy from `linkdean-infra`

- [ ] **Claude, with your OK for the host change:** register a second Actions runner on Nova for `linkdean-infra`. The current runner keeps serving `linkdean` until 1d.
- [ ] **Claude:** add `deploy.yml` in `linkdean-infra`, run by hand with a `version` input. It does four things:
  1. Copies `linkdean-infra`'s `deploy/public-beta/` into the guest at the same path (`/opt/linkdean/deploy/public-beta`). This keeps the `.env`, the Compose project name and the volumes.
  2. Renders the env from Infisical.
  3. Runs `pull-update.sh` with that release tag.
  4. Runs `verify.sh`.

  The guest stops checking out the public repo.
- [ ] **Test:** deploy 0.0.11 again through it. Expect no change, then confirm `/api/health` reports 0.0.11.

### 1d. Clean the public repo

- [ ] **Claude:** one pull request in `linkdean` that removes:
  - `apps/site`, `apps/docs` and the brand working files.
  - `deploy/public-beta/` and `docs/ops/`.
  - The CI jobs `docs-site` and `site`.
  - The release jobs `changes`, `docs-build`, `site-build`, `deploy` and `verify-public`.

  The same pull request also:
  - Moves the OpenAPI spec and the README screenshots.
  - Points the README, `docs/README.md`, `architecture.md`, `features.md`, `operations.md` and the roadmap at what stays.
  - Moves the roadmap items about running linkdean.app to `linkdean-infra`.
  - Tidies `.gitignore`, `.imgbotconfig` and `.graphifyignore`.

  The `docs` CI job (seccomp sync check) and `deploy/docker/` stay. The past entries in `CHANGELOG.md` stay as history.
- [ ] **You:** review and merge.
- [ ] **Claude, with your OK:** remove the old runner from `linkdean`, unless your personal instance still uses it.

Phase 1 is done when:

- The public repo has no site, docs-site, deploy or managed-service files.
- The release workflow ends at the GitHub release.
- 0.0.12 reaches the homelab through `linkdean-infra`.

## Phase 2: product changes in `linkdean`

Each change is its own pull request with tests. They ship together as 0.0.12 to the homelab. They are useful to self-hosters too, so they belong in the public repo.

- [ ] **Claude: real client address behind Cloudflare.** On Google Cloud, the last `X-Forwarded-For` entry is Google's front end or the VM, not the visitor, so every visitor would share one rate-limit bucket. Add an opt-in `CLIENT_IP_HEADER` (set to `CF-Connecting-IP` here), which `client_ip` in `crates/api/src/rate_limit.rs` reads instead. It is only safe where nothing but the tunnel can reach the API, which Phase 6 guarantees.
- [ ] **Claude: egress proxy on its own.** Chrome reaches the web only through the API's egress proxy, which today runs inside the API process (`crates/api/src/main.rs`). On Google Cloud, Chrome sits on the VM and cannot reach Cloud Run on port 3128. Add an `egress-proxy` mode that runs only the proxy, with the same public-address guard. The VM runs it from the same image.
- [ ] **Claude: configurable connection pool.** The pool is fixed at 20 connections (`crates/db/src/lib.rs`), close to the roughly 25 that `db-f1-micro` allows. Add `DATABASE_MAX_CONNECTIONS` (default 20) and set it to 10 on Google Cloud.
- [ ] **Claude: drop page view counting.** Remove:
  - The GoatCounter beacon from the web app (`apps/web/src/lib/analytics/`).
  - The `GOATCOUNTER_URL` setting.
  - Its security-header entry.

  The docs and marketing site beacons are removed in `linkdean-sites`. The Privacy Policy section goes in Phase 7, so users see one re-accept prompt, not two.
- [ ] **Needs your yes: legal pages from instance config.** See the section below.
- [ ] **You:** merge. I then tag 0.0.12 and deploy it to the homelab from `linkdean-infra`.

### Legal pages from instance config

The Terms of Service and Privacy Policy are hard-coded in `apps/web/src/pages/legal/`, and so are the operator details in `operator.ts`. They describe linkdean.app: its operator, iCloud Mail, Cloudflare and GoatCounter. That is managed-service content in the app repo. It also means every self-hosted instance names you as its operator and data controller, which `operator.ts` already records as a known issue.

The proposed change:

- **Default text.** The app ships an operator-neutral version of both pages.
- **Instance files.** An instance can provide its own Terms and Privacy Policy as Markdown files in `LEGAL_DIR`, each with a version date. The terms version users accept comes from those files, instead of `TERMS_VERSION` in `crates/core/src/legal/mod.rs`.
- **Where linkdean.app's files live.** In `linkdean-infra`. Cloud Run reads them from the bucket mount, and the homelab reads them from a bind mount.

The change is medium-sized. Its payoff is that the Phase 7 legal update becomes a file change in `linkdean-infra`, with no app release.

If you would rather not, the legal pages stay in the app repo as the one exception, and Phase 7 edits them there.

### Already fine, no change needed

- **Startup jobs.** Housekeeping runs at startup and then hourly (`crates/api/src/maintenance.rs`), and migrations run at startup (`crates/db/src/lib.rs`). Both suit an instance that starts on demand.
- **Metadata server.** The link-fetch guard already refuses `169.254.169.254` and `metadata.google.internal`, so saved links cannot reach Google's metadata server (`crates/worker/src/net_guard.rs`).

## Phase 3: OpenTofu, Cloudflare and email

These changes leave the app on the homelab untouched.

- [ ] **You:** create the state bucket and the KMS key (manual-setup.md, step 11).
- [ ] **Claude:** set up the first OpenTofu root module in `linkdean-infra`:
  - **Backend:** the GCS bucket, with state and plan encryption through the `gcp_kms` key provider.
  - **Cloudflare:** import today's DNS records and the two Pages projects and their domains, so nothing changes by accident.
  - **AWS SES in `eu-west-2`:**
    - A domain identity for `linkdean.app`, with Easy DKIM (three CNAME records in Cloudflare).
    - A custom MAIL FROM domain, `bounce.linkdean.app` (an MX record and an SPF record on that subdomain).
    - A check of the existing DMARC record.
    - The account-level suppression list on, so addresses that bounce or complain are not mailed again.

  iCloud stays your inbox: the MX records and the apex SPF record do not change.
- [ ] **You:** run `tofu plan`, read it, then `tofu apply` (manual-setup.md, step 12). Expect only additions for SES, and no changes to the imported records.
- [ ] **You:** request SES production access (manual-setup.md, step 13). New accounts can only send to verified addresses until AWS approves this, so start it as soon as the identity is verified.
- [ ] **Claude, with your OK, once 0.0.12 is live:** stop GoatCounter on Nova and remove `stats.linkdean.app` through OpenTofu.

## Phase 4: Google Cloud foundation

I write the OpenTofu, and you review the plan and run `tofu apply` from your laptop. CI runs only `tofu plan`, with a read-only account. Before I run any `gcloud` or `tofu apply` myself, I show you the command and wait for your yes.

- [ ] **Network:**
  - A VPC with two subnets in `europe-west2`: one for the VM (Private Google Access on) and a small one for Cloud Run's outbound traffic.
  - Cloud NAT gives the VM internet access without a public address.
  - Firewall: nothing inbound from the internet. Meilisearch (7700) and Chrome (9222) accept traffic only from the Cloud Run subnet. SSH comes only through Google's IAP range.
- [ ] **Postgres:**
  - Cloud SQL for PostgreSQL 16, matching the homelab's `postgres:16-alpine`.
  - `db-f1-micro`, single zone, 10 GB SSD that grows on its own.
  - Private IP only, with encrypted connections required.
  - Daily backups kept 7 days, point-in-time recovery for 7 days, and deletion protection on.
  - The `linkdean` database.
- [ ] **You:** create the database user and the `DATABASE_URL` secret yourself, so the password never touches the state file (manual-setup.md, step 14).
- [ ] **Media and images:**
  - A private bucket for avatars and previews: public access blocked, 7-day soft delete.
  - An Artifact Registry repository for the app image that keeps the last 5 versions.
- [ ] **Secrets:** empty Secret Manager secrets for every secret value the API and VM read. See manual-setup.md, steps 14 to 18.
- [ ] **Tunnel:** the `linkdean-gcp` Cloudflare tunnel and its routes.
  - OpenTofu creates the tunnel without setting its secret and never reads its token, and I check with `tofu state show` that no token is stored.
  - You copy the token into Secret Manager (manual-setup.md, step 15).
- [ ] **Identities:**
  - One service account for the API: only its own secrets and the media bucket.
  - One for the VM: only the Meilisearch key and the tunnel token.
  - One for the deploy workflow: deploy this one service and push images.

  GitHub signs in through Workload Identity Federation limited to `arsamsarabi/linkdean-infra`, so there are no key files.
- [ ] **Monitoring:**
  - Uptime checks every minute on `app.linkdean.app/api/health`, `docs.linkdean.app` and `linkdean.app`, with an alert to your email and the Google Cloud phone app when one fails.
  - An alert when the database disk passes 80%.
  - The billing budget from Phase 0, now in code.

## Phase 5: the search VM

One `e2-small` VM, with a separate data disk for Meilisearch, so the VM can be rebuilt without losing the index. It runs a small Compose file from `linkdean-infra`, the same shape as the homelab:

- **Meilisearch v1.11**, the version the homelab pins, on the data disk.
- **Chrome:** `chromedp/headless-shell:156.0.8078.36` with the homelab's seccomp profile, on an internal-only network. Its only way out is the egress proxy, as today.
- **Egress proxy:** the app image in its new `egress-proxy` mode, on both the internal network and the outside one.
- **Port forwarder:** publishes Chrome's DevTools port on the VM's private address for Cloud Run. Chrome itself stays on the internal network.
- **cloudflared** for `linkdean-gcp`. It sends `app-next.linkdean.app` (and, at cutover, `app.linkdean.app`) to the Cloud Run URL, with the Host header and TLS name set to that URL.
- **Hourly wake-up:** a timer calls the API's health route every hour, so housekeeping runs even when nobody is using the app.
- **Hardening:** no public IP, OS Login, Shielded VM, a 2 GB swap file, and Docker restarting every container after a reboot.

Cloud Run's ingress is set to internal, so the only way to reach the API is through this tunnel. Google treats requests from a VM in the same project as internal. That is also what makes `CLIENT_IP_HEADER` safe to trust.

## Phase 6: Cloud Run and a rehearsal

- [ ] **Claude:** the `linkdean-api` Cloud Run service in OpenTofu, with:
  - 1 vCPU, 1 GiB, at most 1 instance, scaling to zero, CPU kept on while up, and startup CPU boost.
  - Internal ingress, and outbound traffic to private addresses through the VPC and to the internet directly.
  - The media bucket mounted at the avatar and preview folders.
  - `LOG_FORMAT=json`, `CLIENT_IP_HEADER=CF-Connecting-IP` and `DATABASE_MAX_CONNECTIONS=10`.
  - SMTP set to `email-smtp.eu-west-2.amazonaws.com`, port 587, `SMTP_TLS=starttls`.
  - A startup check on `/api/health`, and secrets read from Secret Manager.
- [ ] **Claude:** `deploy.yml` in `linkdean-infra` gains a Google Cloud target. Given a version, it copies `ghcr.io/arsamsarabi/linkdean:<version>` into Artifact Registry with `crane copy`, deploys it to Cloud Run and checks the version the health route reports.
- [ ] **You:** copy the remaining secrets into Secret Manager (manual-setup.md, steps 17 and 18).
- [ ] **You:** put Cloudflare Access in front of `app-next.linkdean.app`, and add that host to the Turnstile widget (manual-setup.md, steps 19 and 20).

### Rehearsal with real data

This is a full dry run while the homelab keeps serving users. It proves every step and times the real window.

1. Dump the homelab database (`pg_dump -Fc`) and copy the avatar and preview folders, with no downtime.
2. Restore the dump into Cloud SQL from the VM, and sync the folders into the bucket.
3. Deploy with `APP_URL=https://app-next.linkdean.app` and SMTP left unset, so mail is off and no user gets an email from the rehearsal.
4. Reindex search: Admin → Search → Reindex (`POST /api/admin/search/reindex`).
5. Check each of these:
   - Sign in with a password.
   - Unlock a vault, and search.
   - Save a link, and check its title, icon, preview and screenshot.
   - Upload an avatar.
   - Save from the extension.
   - Check that rate limits count your own address.

   Passkeys and Google sign-in will not work on `app-next`, because they are tied to `app.linkdean.app`. They are checked on cutover day instead.
6. Note for Phase 7: which address Cloud Run's request logs record for a visitor. Requests that arrive from the VM may show `0.0.0.0`.
7. Note for Phase 8: how long steps 1 to 4 took, and the first response time after 20 idle minutes.
8. Wipe the copy: drop the database, empty the bucket, remove `app-next`, its Access application and its Turnstile hostname.

## Phase 7: legal pages and the release

The legal update is merged in the week before cutover. Depending on your choice in Phase 2, it is either a file change in `linkdean-infra` or a pull request in `linkdean`. 0.1.0 is the first version to run on Google Cloud.

- [ ] **Claude: Privacy Policy.**
  - Remove the page view statistics section, and fix every "see section N" reference that moves.
  - **Who processes data:**
    - Google Cloud hosts the app and its data (Cloud Run, Cloud SQL, Cloud Storage, Compute Engine).
    - Cloudflare Pages serves the docs and marketing sites.
    - Amazon SES replaces iCloud Mail for sending.
    - Meilisearch now runs on our own VM in Google Cloud.
  - **International transfers:**
    - Data is stored in London.
    - Google, Amazon and Cloudflare are US companies, and their data processing terms apply.
    - Cloudflare serves the sites from its global network.

    I confirm each provider's current UK transfer terms before writing these lines.
  - **Security and retention:**
    - Whether request logs hold IP addresses (from the rehearsal), and that logs are kept 30 days.
    - Database backups are kept 7 days, plus 7 days of point-in-time recovery.
    - Deleted avatars and previews can be recovered for 7 days.
- [ ] **Claude:** bump the terms version to the cutover date, so everyone sees one re-accept prompt.
- [ ] **You:** merge. I then tag 0.1.0 the day before cutover, so the image is already built when the window starts.

## Phase 8: cutover day

The window is about 30 minutes, or whatever the rehearsal measured.

### Before the day

- [ ] **You:** tell users the date and the window at least 3 days ahead.
- [ ] **Claude:** a maintenance page on the guest's Caddy for `app.linkdean.app`, ready to switch on.
- [ ] **You:** check that SES production access is approved and the SMTP values are in Secret Manager.
- [ ] **You:** have the rehearsal notes open: the steps, the timings and the commands that worked.

### In the window

1. **Maintenance on.** Switch the guest's Caddy to the maintenance page and stop the API container (`docker compose stop api`), so nothing writes to the database.
2. **Dump.** Run `pg_dump -Fc` from the database container and take a checksum. Archive the avatar and preview folders.
3. **Move.** Upload both to a private transfer bucket, then download them on the VM.
4. **Restore** into a fresh, empty database, not the rehearsal's: `pg_restore --no-owner --role=linkdean`.
5. **Media.** Sync the folders into the media bucket.
6. **Compare** row counts of users, bookmarks, folders and sessions between the homelab and Cloud SQL. Any difference stops the cutover.
7. **Deploy** 0.1.0 to Cloud Run with `gh workflow run deploy.yml -R arsamsarabi/linkdean-infra -f version=0.1.0 -f target=gcp`, with the real `APP_URL` and mail on. Startup applies any migrations to the restored data.
8. **Reindex** search from Admin.
9. **Switch DNS.** Point `app.linkdean.app` at the `linkdean-gcp` tunnel: one variable in OpenTofu, then `tofu apply`. The record stays proxied by Cloudflare, so the change takes effect in seconds.
10. **Smoke test** as yourself:
    - Sign in with a password and with a passkey, and with Google if you use it.
    - Accept the new terms.
    - Open the library, unlock a vault, and search.
    - Save a link with a screenshot, and see your avatar.
    - Request a password reset and receive it through SES.
    - Save from the extension, and open the admin pages.
11. **Watch** the logs and the uptime checks for an hour.

## Phase 9: watch, roll back if needed, retire the homelab beta

### Rollback

The homelab database is untouched from step 1 of the window, so going back is cheap until users write data on Google Cloud.

- **During the window, or before anyone has used the new setup:** point `app.linkdean.app` back at the homelab tunnel, switch off the maintenance page and start the API container again.
- **After users have written data on Google Cloud:** going back means the same move in reverse (dump Cloud SQL, restore on the guest). Decide within the first day. After that, fix forward.
- **Keep the guest stopped but intact for 14 days.**

### Retire the homelab beta (after 14 quiet days)

- [ ] **You:**
  - Delete the `linkdean-public` LXD guest on Nova.
  - Delete the Backblaze B2 bucket `linkdean-public`, with its restic keys and schedule.
  - Delete the Infisical `/linkdean-public` folder.
  - Remove the public beta's Uptime Kuma monitors.

  See manual-setup.md, step 22.
- [ ] **Claude:**
  - Delete the old tunnel and its DNS records through OpenTofu.
  - Remove the homelab target from `deploy.yml` and `deploy/public-beta/` from `linkdean-infra`.
  - Rewrite the managed-service notes for the Google Cloud setup.
  - Mark the migration done.
- [ ] **Claude, with your OK:** remove the `linkdean-infra` runner from Nova, unless something else uses it.

Your personal instance at `linkdean.arsam.app` is not touched by any of this.

### Later, not part of this move

- **Staging:** a second OpenTofu variables file and its own small database.
- **More than one API instance:** the roadmap's four items (shared unlocked-vault keys, shared rate limits, object storage in code, background work as separate jobs).
- **Off-site backups:** a weekly encrypted export to Backblaze B2, so backups do not live only in the same Google project.
- **Committed use discounts,** once the monthly bill is steady.

## Risks and open questions

| Risk | What it means | Way out |
| --- | --- | --- |
| Scaling to zero | Unlocked vaults lock when the idle instance stops, usually after about 15 minutes with no requests. The first request after that waits for a cold start, which the rehearsal measures | Set the minimum to 1 instance, for about $45 to $55 more a month |
| One VM carries the tunnel, search and screenshots | If the VM is down, the whole app is unreachable, not just search | Docker restarts everything after a reboot, the VM can be rebuilt from code in minutes, and a reindex rebuilds the search index |
| `db-f1-micro` is small | Shared CPU, 0.6 GB of memory, and no Google uptime commitment | Moving up to `db-g1-small` is one setting and a short restart |
| The media folder is a Cloud Storage mount | It behaves like a disk for whole-file writes and reads, which is how avatars and previews are saved. It has no file locks and lists slowly | Proven in the rehearsal. Object storage in code comes with the multi-instance work |
| SES sandbox and reputation | Until AWS approves production access, SES only sends to verified addresses. A high bounce or complaint rate can get sending paused | Request access early in Phase 3. The suppression list stops repeat bounces. Watch the reputation page in the SES console |
| Backups live in the same Google project | A lost or compromised account takes the backups with it | Later: a weekly encrypted export to B2 |
| GitHub Actions minutes in private repos | Private repos use the account's included minutes (2,000 a month on GitHub Free). The site builds and deploys use a few minutes each | Watch usage in the first month. Homelab deploys run on the Nova runner and use no minutes |
| Prices | The cost figures come from search results and published tier 1 rates, not the London price list | The Phase 0 check in the pricing calculator |

## Sources

These are from search results and pages read on 2026-11-15. I confirm the prices again in Phase 0.

- [Cloud Run ingress](https://docs.cloud.google.com/run/docs/securing/ingress): requests from VMs in the same project count as internal.
- [Cloud Run custom domains](https://docs.cloud.google.com/run/docs/mapping-custom-domains): the regions that offer domain mapping, and its preview status.
- [Cloud Run pricing](https://cloud.google.com/run/pricing): tier 1 and tier 2 regions, and the free tier.
- [Google Cloud Observability pricing](https://cloud.google.com/products/observability/pricing): the uptime check allowance and the planned alerting charges.
- [Spare Cores: db-f1-micro](https://sparecores.com/database/gcp/db-f1-micro): the London hourly price for PostgreSQL.
- [Amazon SES pricing](https://aws.amazon.com/ses/pricing/): new SES accounts start on the Essentials plan from 21 July 2026, at $0.16 per 1,000 emails with no fixed fee.
- [AWS account plans](https://docs.aws.amazon.com/awsaccountbilling/latest/aboutv2/free-tier-plans.html): a Free plan account closes after six months or when its credits run out. A Paid plan account does not.
- [SES SMTP endpoints](https://docs.aws.amazon.com/ses/latest/dg/smtp-connect.html): STARTTLS on 587 and TLS on 465.
- [OpenTofu state encryption](https://opentofu.org/docs/language/state/encryption/): the `gcp_kms` key provider.
- [Cloud Billing budgets](https://docs.cloud.google.com/billing/docs/how-to/budgets): budgets alert but do not cap spending.
