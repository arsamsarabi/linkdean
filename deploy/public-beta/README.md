# Public beta deploy (Nova LXD guest)

Isolated stack for `linkdean.app`, `app.linkdean.app`, and `docs.linkdean.app`. Design: [public-beta-homelab-design](../../docs/superpowers/specs/2026-11-06-public-beta-homelab-design.md). Operator checklist: [MANAGED_SERVICE.md](../../docs/MANAGED_SERVICE.md).

Do **not** run this Compose project on the Nova host next to personal services. Use a dedicated LXD VM.

## Prerequisites

- Nova with LXD 5.21+ (`snap list lxd`). If LXD was never initialized, run `sudo lxd init` **only after** an explicit go-ahead.
- Cloudflare zone for `linkdean.app` (Tunnel + DNS created below).
- A GitHub release / GHCR image tag for `ghcr.io/arsamsarabi/linkdean`.

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

## 4. CI / release / selective updates

| Channel | Trigger | What it publishes |
| --- | --- | --- |
| [CI](../../.github/workflows/ci.yml) | PR + push to `main` | Tests; site build must match committed `static/site`; docs build uploads a Linux artifact |
| [Release](../../.github/workflows/release.yml) | Tag `v*.*.*` | GHCR `:x.y.z`, `:x.y`, `:latest` + GitHub release |
| [Deploy main](../../.github/workflows/deploy-main.yml) | Push to `main` (path-filtered) | App → GHCR `:main` + `:sha-<short>`; docs → Linux static artifact; site → committed-static check |

**Intelligent updates:** Deploy main uses [dorny/paths-filter](https://github.com/dorny/paths-filter) so only changed deployables run (app crates/web/Docker, docs markdown/`apps/docs`, marketing `apps/site`). Manual **workflow_dispatch** can force any subset (or all if none forced).

**Guest roll-forward (manual):** after `git pull` on the guest:

```bash
# From /opt/linkdean — pulls api image and/or restarts Caddy for static
bash deploy/public-beta/scripts/pull-update.sh
```

For continuous app images set `LINKDEAN_VERSION=main` in `.env`. Pin a release tag for slower upgrades.

**Optional auto-deploy from GitHub Actions:** repository variable `NOVA_DEPLOY_ENABLED=true` plus secrets `NOVA_DEPLOY_HOST`, `NOVA_DEPLOY_USER`, `NOVA_DEPLOY_SSH_KEY`, and usually `NOVA_DEPLOY_LXC=linkdean-public` (SSH to Nova, then `lxc exec`). The host must be reachable from GitHub (Tailscale Funnel/subnet, public SSH, or a self-hosted runner). Leave disabled until that path is intentional.

## 5. Before announce

Complete the go-live list in the design spec / MANAGED_SERVICE: SMTP + SPF/DKIM/DMARC, Turnstile, `REQUIRE_VERIFIED_EMAIL`, `BLOCK_DISPOSABLE_EMAILS`, Chrome off LAN, off-site backups + restore test, external uptime, ISP terms, `LOG_FORMAT=json` (default here).

## Networks

Compose pins `10.89.10.0/24`, `10.89.11.0/24`, `10.89.12.0/24`. If they clash inside the guest, change them and ensure they are free (`docker network ls` / inspect).
