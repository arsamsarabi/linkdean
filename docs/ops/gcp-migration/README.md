# Linkdean: homelab to Google Cloud migration

Status: planned, not started. Last updated 2026-11-15, after a review that cut the design down to one server.

This folder moves to the private `linkdean-infra` repo with the rest of `docs/ops/` in Phase 1. Your hand-done steps are in [manual-setup.md](manual-setup.md), in the order the phases need them.

## Summary

The public beta moves from the Nova LXD guest to Google Cloud in London:

- **App server:** one small VM runs the homelab's Docker Compose setup: the API, Meilisearch, headless Chrome, Caddy and the tunnel.
- **Postgres:** the smallest Cloud SQL tier, with managed backups.

Email moves from iCloud Mail to Amazon SES in London. The docs and marketing sites move to Cloudflare Pages. The Google Cloud resources, and the Cloudflare tunnel and records they need, are defined in OpenTofu.

Before any of that, the public `linkdean` repo is cut down to the app itself. The sites, the deploy code, the legal pages and the managed-service notes move to one private repo, `linkdean-infra`.

The estimated cost is $55 to $80 a month. The move needs one maintenance window of about 30 minutes.

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
| App server | One `e2-medium` VM (2 shared vCPUs, 4 GB), always on. It runs the homelab's Compose file without the database and the static sites: the API, Meilisearch, headless Chrome, Caddy and `cloudflared`. Cloud NAT gives it outbound traffic |
| Postgres | Cloud SQL for PostgreSQL 16, `db-f1-micro`, 10 GB SSD, private IP only. Daily backups kept 7 days, plus 7 days of point-in-time recovery |
| Avatars and previews | Docker volumes on the VM's data disk, with a daily snapshot kept 7 days |
| How traffic reaches the API | A new Cloudflare tunnel, `linkdean-gcp`, runs on the VM. The VM has no public address and accepts no inbound traffic |
| Email | Amazon SES in `eu-west-2` (London), on the Essentials plan new accounts start on. Set up once by hand in the SES console |
| Docs and marketing sites | Cloudflare Pages. Files are uploaded from `linkdean-infra` |
| Legal pages | Move to the marketing site. The app links to them through three settings |
| Infrastructure as code | OpenTofu, for Google Cloud and the Cloudflare tunnel and records it adds. State lives in a private, versioned Cloud Storage bucket |
| Repos | `linkdean` (public, the app only) and `linkdean-infra` (private, everything else) |
| Deploys | One `gh workflow run` in `linkdean-infra` per release. It runs on a self-hosted runner: on Nova until cutover, then on the VM. The public repo has no deploy step |
| Page view statistics | Dropped. The GoatCounter beacon and `stats.linkdean.app` go |
| Monitoring | Cloud Monitoring uptime checks on the app, docs and site. Alerts go to email and the Google Cloud phone app |
| Environments | Production only. Staging comes later, as a second OpenTofu variables file |
| Homelab | The personal instance at `linkdean.arsam.app` stays. The public-beta guest, its tunnel and its B2 backups are retired 14 days after cutover |

### Why one VM, not Cloud Run

The first draft ran the API on Cloud Run and everything else on a VM. A review found that the split cost more and bought nothing:

- **Cloud Run would never scale to zero.** Every uptime check runs from at least three places, so the API gets a request about every 20 seconds. Cloud Run keeps an instance for up to 15 minutes after its last request, so it would never stop. That adds the $45 to $55 a month an always-on instance costs.
- **The VM is needed anyway.** Chrome and Meilisearch need a server, and when that server is down the app is unreachable. Cloud Run's managed uptime does not help while the VM it depends on is down.
- **No app changes.** On one server the homelab setup carries over as it is. The split needed three new app features: a stand-alone egress proxy, a trusted client-address header and a configurable connection pool. None of them is needed now.
- **Fewer parts.** There is no Artifact Registry, image copy step, storage mount, port forwarder, wake-up timer or GitHub deploy identity.

The cost is that the VM's operating system must be kept patched (automatic security updates do most of it), and moving to a bigger VM means a short restart.

### Why OpenTofu

- **Staging later is cheap.** It is one more variables file, not another round of console clicks.
- **Every change is a reviewed diff.** `tofu plan` shows what will change before anything does.
- **The Cloudflare side sits with Google.** The tunnel, its routes and the DNS records SES needs live next to the Google resources, so cutover is a one-line change.
- **Rebuilding after a mistake is a command,** not a memory exercise.

Some things stay out of OpenTofu on purpose:

- **Secret values.** OpenTofu creates empty Secret Manager secrets, and you add the values with `gcloud`. The same goes for the database user's password, the tunnel token and the SES SMTP password. The state file therefore holds no secrets, and a private bucket is all the protection it needs.
- **Things made by hand that already work.** Today's DNS records (iCloud Mail's among them), the Pages projects and their domains, the billing budget and the SES setup. Only the `app.linkdean.app` record is imported, because cutover switches it.
- **Which release runs.** The deploy workflow sets it.
- **The one-off data move.** That is a runbook (Phase 8), not infrastructure.

## Monthly cost

These are rough London prices from third-party trackers and published rates. You check them in the pricing calculator in Phase 0.

| Item | Est. $/month |
| --- | --- |
| `e2-medium` VM | 32–52 |
| Boot and data disks, snapshots | 3–5 |
| Cloud NAT | 1–3 |
| Cloud SQL `db-f1-micro` | 16–20 |
| Secret Manager, Logging, Monitoring, state bucket | 0–2 |
| Amazon SES: $0.16 per 1,000 emails on Essentials, no monthly fee | under 1 |
| Cloudflare Pages | 0 |
| **Total** | **about 55–80** (£45–65) |

- **The VM price is the open one.** One tracker lists `e2-medium` in London at $32 a month, another at $52. The calculator settles it.
- **Trial credit.** A Google account that has never used the free trial gets $300 of credit for 90 days. A new AWS account gets $100 of credit.
- **Monitoring is free at this size.** Uptime checks get 1 million runs a month free per project. Three checks every minute from three regions is about 390,000 runs. Alerts on uptime checks are exempt from the alerting charges Google plans to start no sooner than 1 September 2027.

## Repo split

The goal is that `linkdean` holds only what someone needs to build, run or self-host the app. It should hold nothing about how linkdean.app is hosted, deployed, marketed or governed.

| Repo | Holds | Visibility |
| --- | --- | --- |
| `linkdean` | `crates/`, `apps/web`, `apps/extension`, migrations, `deploy/docker/` for self-hosters, the markdown under `docs/` (self-hosting, guides, product, design, CHANGELOG), the OpenAPI spec, README screenshots, and the logo kit (`brand/final/`, `brand/exports/`, `brand/GUIDELINES.md`) | Public |
| `linkdean-infra` | `apps/site` (Astro; it gains the Terms and Privacy Policy in Phase 2), `apps/docs` (Docusaurus, including its own pages such as the intro, getting started, beta, compare and the public roadmap), the brand working files (`brand/board`, `concepts`, `graphics`, `renders`, `node_render`, `HANDOVER.md`), `tofu/`, `deploy/public-beta/` (until Phase 9), `deploy/gcp/`, `docs/ops/` including this plan, the go-live checklist and the roadmap items about running linkdean.app, and the workflows | Private |

Moved folders keep their paths, so their build scripts need only the changes listed in Phase 1.

### How the docs site still follows the code

The docs build checks out `linkdean` at the release tag it is given. The repo is public, so this needs no token. It copies the same markdown and OpenAPI spec as today.

A docs change for a feature still lands in the same `linkdean` pull request as the feature. It goes live with the next deploy after the release. Two files move so the app repo no longer points into `apps/docs`:

- The OpenAPI spec moves from `apps/docs/static/openapi.json` to `docs/api/openapi.json`. The `committed_spec_is_current` test in `crates/api/src/routes/mod.rs` writes it there.
- The README screenshots move from `apps/docs/static/img/screenshots/` to `docs/media/screenshots/`. The docs site copies them from there.

### No deploy hook in the public repo

The public repo has no deploy step, and holds no token that could start one. A token there that can start `linkdean-infra` workflows would make a compromised release job more dangerous.

After a release:

1. **Tag in `linkdean`.** CI runs, the image goes to GHCR, the extension job runs if its version changed, and the GitHub release is created.
2. **Deploy.** `gh workflow run deploy.yml -R arsamsarabi/linkdean-infra -f version=X.Y.Z`. It deploys the app and rebuilds the docs site from that tag. Its `target` input is `homelab` until cutover and `gcp` after it. I run this as part of each release.

The marketing site deploys on its own whenever `apps/site` changes on `main` in `linkdean-infra`.

### History

The moved paths keep their history: they are copied with `git filter-repo` into `linkdean-infra`. Old commits that touched them stay in the public history. Nothing secret was committed there, so this split is about what the repo looks like from now on.

## Phase 0: accounts and tools

All yours; see [manual-setup.md](manual-setup.md). Nothing here costs money yet.

- [ ] Cloudflare token for Pages deploys (needed in Phase 1).
- [ ] Install `gcloud` and `opentofu`.
- [ ] Google Cloud project `linkdean-prod`, with billing and a budget.
- [ ] AWS account on the Paid plan, with root MFA, a budget and an IAM user for the console.
- [ ] Cloudflare token for OpenTofu, stored in Infisical.
- [ ] Price the setup in the Google Cloud pricing calculator.

## Phase 1: repo split

I do the work and open the pull requests. You review, add the Pages custom domains, and approve the one host change.

### 1a. New repo

- [ ] **Claude:** create the private repo `arsamsarabi/linkdean-infra`. You approved this on 2026-11-15.
- [ ] **Claude:** copy the paths in the table above into it with `git filter-repo` from a fresh clone, then push `main`.
- [ ] **Claude, in `linkdean-infra`:**
  - Make the docs sync step read from a `linkdean` checkout (`LINKDEAN_DIR`) instead of `../../docs`.
  - Add a CI workflow that builds whichever site changed on pull requests.
  - Add `deploy-site.yml`, which builds the marketing site and uploads it with `wrangler pages deploy` when `apps/site` changes on `main`.
  - Add `deploy.yml` with a `version` input and one job for now: build the docs from that tag and upload them with `wrangler pages deploy`. The build writes a `release.txt` naming the release, so a check can confirm which build is live.
- [ ] **Claude:** move the local Claude skills with what they cover. These are untracked files under `.claude/skills/`: `linkdean-marketing-site`, `static-docs-deploy-verification`, `linkdean-deployment` and `linkdean-backup-restore-drill` go to `linkdean-infra`.

### 1b. Sites on Cloudflare Pages

This is the one hosting change in Phase 1. Without it, `linkdean-infra` would need a throwaway path back to Nova for a few weeks.

- [ ] **Claude:** create two Pages projects with wrangler, `linkdean-site` and `linkdean-docs`. Then deploy each to its `*.pages.dev` address and compare it page by page with the live site.
- [ ] **Claude:** carry over what the guest's Caddyfile does:
  - **www and apex:** both serve the marketing site, so both domains go on the same project.
  - **Marketing site:** falls back to its index page.
  - **Docs:** serve a pre-rendered page per path and their own 404 page.

  Pages does the last two by default. I check each after the switch.
- [ ] **You:** add the custom domains (manual-setup.md, step 2). Note each old DNS target first: pointing the record back at the homelab tunnel is the rollback.
- [ ] **Check:** `linkdean.app`, `www.linkdean.app` and `docs.linkdean.app` in a private window, a missing page on each, and the docs `release.txt`.

### 1c. Homelab deploy from `linkdean-infra`

- [ ] **Claude, with your OK for the host change:** check read-only whether your personal instance uses the Nova runner. If it does not, re-register that runner from `linkdean` to `linkdean-infra`, keeping its `linkdean-nova` label. If it does, register a second runner for `linkdean-infra` instead.
- [ ] **Claude:** set the `linkdean` repo variable `NOVA_DEPLOY_ENABLED` to `false`, so its release workflow skips the old deploy job until 1d removes it.
- [ ] **Claude:** add an `app` job to `deploy.yml`, run on the Nova runner. It does four things:
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
- [ ] **Claude:** delete the `NOVA_DEPLOY_ENABLED` variable.

Phase 1 is done when:

- The public repo has no site, docs-site, deploy or managed-service files.
- The release workflow ends at the GitHub release.
- 0.0.12 reaches the homelab through `linkdean-infra`.

## Phase 2: product changes in `linkdean`

Each change is its own pull request with tests. They ship together as 0.0.12 to the homelab.

- [ ] **Claude: drop page view counting.** Remove:
  - The GoatCounter beacon from the web app (`apps/web/src/lib/analytics/`).
  - The `GOATCOUNTER_URL` setting.
  - Its security-header entry.

  The docs and marketing site beacons are removed in `linkdean-infra`. The Privacy Policy section goes in Phase 7, so users see one re-accept prompt, not two.
- [ ] **Claude: legal pages move out of the app.** Today the Terms and Privacy Policy are hard-coded in `apps/web/src/pages/legal/`, with the operator details in `operator.ts`. They describe linkdean.app, and every self-hosted instance names you as its operator. The change:
  - **Three settings.** `TERMS_VERSION`, `TERMS_URL` and `PRIVACY_URL`. The terms check at sign-up and the re-accept prompt use `TERMS_VERSION` instead of the constant in `crates/core/src/legal/mod.rs`. An instance without it asks for no acceptance and shows no legal links. A test covers both cases.
  - **Web app.** It reads the version and links from the sign-in config the API already serves (`crates/api/src/routes/auth/mod.rs`), and drops its own copy of the version in `apps/web/src/lib/legal/terms.ts`.
  - **Old links keep working.** `/terms`, `/privacy` and `/acceptable-use` on the app become redirects to the settings' URLs, so links in sent emails and in the extension store listings still land on the right page.
  - **Deleted:** `apps/web/src/pages/legal/` and `operator.ts`, and with them the known issue about self-hosted operators.
  - **In `linkdean-infra`:** the Terms and Privacy Policy move to the marketing site as `linkdean.app/terms` and `linkdean.app/privacy`, with the text unchanged. They go live before 0.0.12 is deployed.
  - **linkdean.app** sets `TERMS_VERSION=2026-11-15`, the current version, so nobody is asked to accept again.
- [ ] **You:** merge. I then tag 0.0.12 and deploy it to the homelab from `linkdean-infra`.

### Already fine, no change needed

- **Real client address.** Caddy stays in front of the API, with the homelab setting that takes the visitor's address from Cloudflare's `CF-Connecting-IP` header (`deploy/public-beta/Caddyfile`). Only the tunnel can reach Caddy.
- **Egress proxy.** Chrome reaches the web only through the proxy inside the API, on the same internal Docker network as today.
- **Connection pool.** The API's 20 connections fit under the 25 that `db-f1-micro` allows. Compose stops the old API container before it starts the new one, so two pools never overlap.
- **Startup jobs.** Housekeeping runs at startup and then hourly (`crates/api/src/maintenance.rs`), and migrations run at startup (`crates/db/src/lib.rs`). The API never stops, so housekeeping keeps its schedule.
- **Metadata server.** The link-fetch guard already refuses `169.254.169.254` and `metadata.google.internal`, so saved links cannot reach Google's metadata server (`crates/worker/src/net_guard.rs`).

## Phase 3: OpenTofu, Cloudflare and email

These changes leave the app on the homelab untouched.

- [ ] **You:** create the state bucket (manual-setup.md, step 11).
- [ ] **You:** create the SES identity for `linkdean.app` in the SES console, and send me its three DKIM records (step 13). They are not secret.
- [ ] **Claude:** set up the first OpenTofu root module in `linkdean-infra`:
  - **Backend:** the state bucket.
  - **Cloudflare:** import only the `app.linkdean.app` record, so cutover can switch it.
  - **SES records:** the three DKIM CNAME records, plus an MX and an SPF record for the MAIL FROM subdomain `bounce.linkdean.app`.
  - **DMARC:** a check of the existing record.

  iCloud stays your inbox: the MX records and the apex SPF record are not touched, and are not in OpenTofu.
- [ ] **You:** run `tofu plan`, read it, then `tofu apply` (step 12). Expect only additions, and no change to the imported record.
- [ ] **You:** request SES production access once the identity shows as verified (step 14). New accounts can only send to verified addresses until AWS approves this.
- [ ] **Claude, with your OK, once 0.0.12 is live:** stop GoatCounter on Nova, and remove the DNS record and tunnel route for `stats.linkdean.app`.

## Phase 4: Google Cloud foundation

I write the OpenTofu, and you review the plan and run `tofu apply` from your laptop (step 12). Before I run any `gcloud` command or `tofu apply` myself, I show you the command and wait for your yes.

- [ ] **Network:**
  - A VPC with one subnet in `europe-west2` for the VM, with Private Google Access on.
  - A private services access range, so Cloud SQL gets an address inside the VPC.
  - Cloud NAT gives the VM internet access without a public address.
  - Firewall: nothing inbound from the internet. SSH comes only through Google's IAP range.
- [ ] **Postgres:**
  - Cloud SQL for PostgreSQL 16, matching the homelab's `postgres:16-alpine`.
  - `db-f1-micro`, single zone, 10 GB SSD that grows on its own.
  - Private IP only, with encrypted connections required.
  - Daily backups kept 7 days, point-in-time recovery for 7 days, and deletion protection on.
  - The `linkdean` database.
- [ ] **You:** create the database user and the `DATABASE_URL` secret yourself, so the password never touches the state file (step 15).
- [ ] **Secrets:** empty Secret Manager secrets for every secret value the server reads. See steps 15 to 19.
- [ ] **Tunnel:** the `linkdean-gcp` Cloudflare tunnel, with routes to Caddy on the VM.
  - OpenTofu creates the tunnel without setting its secret and never reads its token, and I check with `tofu state show` that no token is stored.
  - You copy the token into Secret Manager (step 16).
- [ ] **Identity:** one service account for the VM. It can read its own secrets and write logs and metrics, nothing else. It has no key file, and there is no deploy identity: deploys run on the VM's own runner.
- [ ] **Monitoring:**
  - Uptime checks every minute on `app.linkdean.app/api/health`, `docs.linkdean.app` and `linkdean.app`, with an alert to your email and the Google Cloud phone app when one fails.
  - An alert when the database disk passes 80%.

## Phase 5: the VM

One `e2-medium` VM (2 shared vCPUs, 4 GB) on Debian 12. A separate data disk holds the Docker volumes, so the VM can be rebuilt without losing avatars, previews or the search index. A daily snapshot of that disk is kept 7 days.

It runs `deploy/gcp/docker-compose.yml` from `linkdean-infra`: the homelab file without the database and the static sites.

- **API:** `ghcr.io/arsamsarabi/linkdean:<version>`, with `DATABASE_URL` pointing at Cloud SQL (`sslmode=require`). Chrome's egress proxy runs inside it, as today.
- **Meilisearch v1.11**, the version the homelab pins, on the data disk.
- **Chrome:** `chromedp/headless-shell:156.0.8078.36` with the homelab's seccomp profile, on an internal-only network. Its only way out is the egress proxy, as today.
- **Caddy** for `app.linkdean.app` (and `app-next.linkdean.app` for the rehearsal), with the homelab's client-address settings.
- **cloudflared** for `linkdean-gcp`.

Setup and hardening:

- **Bootstrap.** `deploy/gcp/bootstrap.sh` installs Docker and a GitHub Actions runner for `linkdean-infra` with the label `linkdean-gcp`. I run it over IAP SSH with your OK. Rebuilding the VM is `tofu apply` and this script.
- **Runner rule.** As on Nova, only `deploy.yml` uses the `linkdean-gcp` label, never a pull-request job.
- **Hardening:** no public IP, OS Login, Shielded VM, automatic security updates, a 2 GB swap file, and Docker restarting every container after a reboot.

## Phase 6: deploy target and rehearsal

- [ ] **Claude:** `deploy.yml` gains a `gcp` target that runs on the VM's runner. It writes the `.env` from Secret Manager, pulls the given release, restarts the API and checks the version the health route reports. The settings that are not secret live in `deploy/gcp/`:
  - `APP_URL`, `LOG_FORMAT=json`, and the three legal settings.
  - SMTP: `email-smtp.eu-west-2.amazonaws.com`, port 587, `SMTP_TLS=starttls`.
- [ ] **You:** copy the remaining secrets into Secret Manager (steps 18 and 19).
- [ ] **You:** put Cloudflare Access in front of `app-next.linkdean.app` (step 20).

### Rehearsal with real data

This is a full dry run while the homelab keeps serving users. It proves every step and times the real window.

1. Dump the homelab database (`pg_dump -Fc`) and archive the avatar and preview folders, with no downtime.
2. Copy both to the VM through your Mac: from Nova with `scp`, then to the VM with `gcloud compute scp --tunnel-through-iap`. The guest needs no Google login, and no transfer bucket is needed.
3. Restore the dump into Cloud SQL from the VM, and unpack the folders into the VM's volumes.
4. Deploy with `APP_URL=https://app-next.linkdean.app` and SMTP left unset, so mail is off and no user gets an email from the rehearsal.
5. Reindex search: Admin → Search → Reindex (`POST /api/admin/search/reindex`).
6. Check each of these:
   - Sign in with a password.
   - Unlock a vault, and search.
   - Save a link, and check its title, icon, preview and screenshot.
   - Upload an avatar.
   - Save from the extension.
   - Check that rate limits count your own address.

   Passkeys and Google sign-in will not work on `app-next`, because they are tied to `app.linkdean.app`. They are checked on cutover day instead.
7. Note for Phase 8: how long steps 1 to 5 took.
8. Wipe the copy: drop the database, clear the volumes and the search index, and remove the `app-next` route and its Access application.

## Phase 7: legal update

The Terms and Privacy Policy live on the marketing site in `linkdean-infra` since Phase 2. The update is merged in the week before cutover and goes live in the window.

- [ ] **Claude: Privacy Policy.**
  - Remove the page view statistics section, and fix every "see section N" reference that moves.
  - **Who processes data:**
    - Google Cloud hosts the app and its data (Compute Engine and Cloud SQL).
    - Cloudflare Pages serves the docs and marketing sites.
    - Amazon SES replaces iCloud Mail for sending.
    - Meilisearch runs on our own server in Google Cloud.
  - **International transfers:**
    - Data is stored in London.
    - Google, Amazon and Cloudflare are US companies, and their data processing terms apply.
    - Cloudflare serves the sites from its global network.

    I confirm each provider's current UK transfer terms before writing these lines.
  - **Security and retention:**
    - Request logs stay on the server, as on the homelab. I check that the current wording still holds.
    - Database backups are kept 7 days, plus 7 days of point-in-time recovery.
    - Snapshots of avatars and previews are kept 7 days.
- [ ] **Claude:** set `TERMS_VERSION` in `deploy/gcp/` to the cutover date, so everyone sees one re-accept prompt.
- [ ] **You:** merge.

No special app release is needed: cutover deploys the newest release.

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
3. **Move** both to the VM through your Mac, as in the rehearsal, and check the checksum.
4. **Restore** into a fresh, empty database, not the rehearsal's: `pg_restore --no-owner --role=linkdean`.
5. **Media.** Unpack the folders into the VM's volumes.
6. **Compare** row counts of users, bookmarks, folders and sessions between the homelab and Cloud SQL. Any difference stops the cutover.
7. **Deploy** the newest release with `gh workflow run deploy.yml -R arsamsarabi/linkdean-infra -f version=X.Y.Z -f target=gcp`, with the real `APP_URL`, mail on and the new `TERMS_VERSION`. Startup applies any migrations to the restored data. The marketing site deploys with the updated legal pages from the Phase 7 merge.
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
  - Delete the old public-beta tunnel in Cloudflare.

  See manual-setup.md, step 22.
- [ ] **Claude:**
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
| One VM carries the whole app | If the VM is down, the app is unreachable | Docker restarts everything after a reboot. The VM can be rebuilt from code in minutes, and the data sits in Cloud SQL and on the snapshotted data disk |
| The VM runs out of memory | 4 GB holds the API, Chrome (capped at 1 GB) and Meilisearch, with swap for spikes | Moving to `e2-standard-2` (8 GB) is one setting and a short restart |
| `db-f1-micro` is small | Shared CPU, 0.6 GB of memory, no Google uptime commitment, and 25 connections, of which the API uses up to 20 | Moving up to `db-g1-small` is one setting and a short restart |
| A self-hosted runner on the production VM | Any job sent to its label runs on the server that holds the app | The repo is private and only you push to it. Only `deploy.yml` uses the label, never a pull-request job |
| SES sandbox and reputation | Until AWS approves production access, SES only sends to verified addresses. A high bounce or complaint rate can get sending paused | Request access early in Phase 3. The suppression list stops repeat bounces. Watch the reputation page in the SES console |
| Backups live in the same Google project | A lost or compromised account takes the backups with it | Later: a weekly encrypted export to B2 |
| GitHub Actions minutes in a private repo | Private repos use the account's included minutes (2,000 a month on GitHub Free). The site and docs builds use a few minutes each | Watch usage in the first month. App deploys run on self-hosted runners and use no minutes |
| Prices | The trackers disagree on the VM price, and none of the figures come from Google's London price list | The Phase 0 check in the pricing calculator |

## Sources

These are from search results and pages read on 2026-11-15. I confirm the prices again in Phase 0.

- [Uptime checks](https://docs.cloud.google.com/monitoring/uptime-checks/introduction): each check needs at least three checkers.
- [Cloud Run instance autoscaling](https://docs.cloud.google.com/run/docs/about-instance-autoscaling): an idle instance is kept for up to 15 minutes after its last request.
- [Google Cloud Observability pricing](https://cloud.google.com/products/observability/pricing): the uptime check allowance and the planned alerting charges.
- [gcloud-compute.com: e2-medium](https://gcloud-compute.com/e2-medium.html) and [Spare Cores: e2-medium](https://sparecores.com/server/gcp/e2-medium): the two London prices for the VM.
- [Spare Cores: db-f1-micro](https://sparecores.com/database/gcp/db-f1-micro): the London hourly price for PostgreSQL.
- [Amazon SES pricing](https://aws.amazon.com/ses/pricing/): new SES accounts start on the Essentials plan from 21 July 2026, at $0.16 per 1,000 emails with no fixed fee.
- [AWS account plans](https://docs.aws.amazon.com/awsaccountbilling/latest/aboutv2/free-tier-plans.html): a Free plan account closes after six months or when its credits run out. A Paid plan account does not.
- [SES SMTP endpoints](https://docs.aws.amazon.com/ses/latest/dg/smtp-connect.html): STARTTLS on 587 and TLS on 465.
- [Cloud Billing budgets](https://docs.cloud.google.com/billing/docs/how-to/budgets): budgets alert but do not cap spending.
