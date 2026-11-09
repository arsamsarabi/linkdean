# Public beta deploy (Nova LXD guest)

Isolated stack for `linkdean.app`, `app.linkdean.app`, and `docs.linkdean.app`. Design: [public-beta-homelab-design](../../docs/superpowers/specs/2026-11-06-public-beta-homelab-design.md). Operator checklist: [MANAGED_SERVICE.md](../../docs/MANAGED_SERVICE.md).

Do **not** run this Compose project on the Nova host next to personal services. Use a dedicated LXD VM.

## Prerequisites

- Nova with LXD 5.21+ (`snap list lxd`). If LXD was never initialized, run `sudo lxd init` **only after** an explicit go-ahead.
- Cloudflare zone for `linkdean.app` (Tunnel + DNS created below).
- A GitHub release / GHCR image tag for `ghcr.io/arsamsarabi/linkdean`.
- The repo cloned at `/opt/linkdean` on the guest (the Release workflow deploys there).

## 1. Create the guest (on Nova)

```bash
# Virtual machine (stronger isolation than a container)
lxc launch ubuntu:24.04 linkdean-public --vm
lxc exec linkdean-public -- bash
```

Inside the guest: install Docker Engine (official docs for Ubuntu), then copy this `deploy/public-beta` directory onto the guest (git clone of the repo, or rsync).

Suggested resources to start: 2–4 vCPUs, 4–8 GiB RAM, enough disk for Postgres + previews.

## 2. Cloudflare Tunnel

1. Zero Trust → Networks → Tunnels → Create a tunnel (Cloudflared).
2. Copy the **tunnel token** into `.env` as `TUNNEL_TOKEN`.
3. Public hostnames (all → `http://caddy:80` on the Docker `edge` network — the token connector reaches Compose by service name when cloudflared runs in this project):

   | Hostname | Service |
   | --- | --- |
   | `linkdean.app` | `http://caddy:80` |
   | `www.linkdean.app` | `http://caddy:80` |
   | `docs.linkdean.app` | `http://caddy:80` |
   | `app.linkdean.app` | `http://caddy:80` |

4. DNS: proxied CNAME/alias records for those names to the tunnel (dashboard can create them).

No router port forwarding.

## 3. Configure and start

```bash
cd deploy/public-beta
cp .env.example .env
# Fill POSTGRES_PASSWORD, MEILI_MASTER_KEY, SESSION_SECRET, TUNNEL_TOKEN (openssl rand -hex 32)
docker compose pull
docker compose up -d
docker compose ps
```

Optional screenshots:

```bash
# Set BROWSER_WS_URL=http://chrome:9222 in .env first
docker compose --profile chrome up -d
```

Smoke: `https://app.linkdean.app/api/health` and the app shell.

Static sites: from the repo root, `pnpm --dir apps/docs build` publishes into `static/docs/` (served at `docs.linkdean.app`). `pnpm --dir apps/site build` publishes the Astro marketing site into `static/site/` (served at `linkdean.app`).

**Do not seed accounts.** First admin is onboarding on an empty instance. Leave `BOOTSTRAP_TOKEN` for the operator if needed.

## 4. Releases and deploys

| Workflow | Trigger | What it does |
| --- | --- | --- |
| [CI](../../.github/workflows/ci.yml) | PR and push to `main` | Tests and builds every app; the site build must match the committed `static/site` |
| [Release](../../.github/workflows/release.yml) | Tag `v*.*.*` on a commit in `main` | Builds and deploys only the apps changed since the previous tag, then checks they are live |

**What counts as changed** (diff from the previous `v*.*.*` tag to this one; the first tag deploys everything):

| App | Paths | Deploy |
| --- | --- | --- |
| app | `crates/`, `apps/web/`, `migrations/`, `deploy/docker/`, `Cargo.toml`, `Cargo.lock` | GHCR `:x.y.z`, `:x.y`, `:latest`; the guest's `LINKDEAN_VERSION` set to the tag, api pulled and restarted |
| docs | `apps/docs/`, `static/docs/`, `docs/INSTALL.md`, `docs/SELF_HOSTED.md`, `docs/FEATURES.md`, `docs/ARCHITECTURE.md` | guest checks out the tag; Caddy restarted |
| site | `apps/site/`, `static/site/` | guest checks out the tag; Caddy restarted |

An app release needs the tag to match the version in `Cargo.toml` and `apps/web/package.json`; a docs- or site-only release does not.

**Cutting a release:**

```bash
git switch main && git pull
git tag v0.2.0 && git push origin v0.2.0
```

**Deploy and checks** run only when the repository variable `NOVA_DEPLOY_ENABLED=true` is set. The deploy job runs on a self-hosted runner labelled `linkdean-nova` on the Nova host and uses `lxc exec linkdean-public` (no SSH secrets). Keep that label off pull_request jobs — this repo is public.

On the guest the job checks out the tag in `/opt/linkdean`, runs [`scripts/pull-update.sh`](scripts/pull-update.sh) for the changed apps, then [`scripts/verify.sh`](scripts/verify.sh), which fails the release unless:

- app: the api container runs the tagged image, is healthy, `/api/health` reports the tag's version and `/api/ready` passes;
- docs and site: Caddy serves the checked-out `index.html` for `docs.linkdean.app` and `linkdean.app`.

A last job then fetches each changed app through Cloudflare (`app.linkdean.app/api/health` must report the new version). The GitHub release is created only after the deploy passes.

Redeploy a release by hand: Actions → Release → Run workflow on the tag; tick "Deploy every app" to refresh all three.

**Manual roll-forward on the guest** (same scripts):

```bash
cd /opt/linkdean
git fetch --tags && git checkout --detach v0.2.0
REF_BEFORE=v0.1.0 RELEASE_TAG=v0.2.0 bash deploy/public-beta/scripts/pull-update.sh   # refreshes what changed since v0.1.0
RELEASE_TAG=v0.2.0 CHECK_APP=true CHECK_DOCS=true CHECK_SITE=true bash deploy/public-beta/scripts/verify.sh
```

## 5. Before announce

Remaining go-live items are tracked in [ROADMAP.md](../../docs/ROADMAP.md) (item 13). Tools for them:

- Sign-up controls: `bash deploy/public-beta/scripts/check-signup-controls.sh` checks Turnstile, the disposable-domain block and outbound mail against the live app, without creating an account.
- Restore drill: [BACKUP.md](./BACKUP.md#restore-drill) (`scripts/restore-drill.sh` on Nova).
- Uptime checks: [MONITORING.md](./MONITORING.md).

Guest `.env` comes from Infisical via [`scripts/render-env-from-infisical.sh`](./scripts/render-env-from-infisical.sh). Marketing contact (`contact@linkdean.app`) is already on the site.

## Networks

Compose pins `10.89.10.0/24`, `10.89.11.0/24`, `10.89.12.0/24`. If they clash inside the guest, change them and ensure they are free (`docker network ls` / inspect).
