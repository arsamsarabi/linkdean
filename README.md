<p align="center">
  <img src="brand/graphics/the-link-dean.png" alt="The Link Dean, Linkdean mascot" width="280" />
</p>

<h1 align="center">Linkdean</h1>

<p align="center">
  <strong>Self-hostable bookmark manager.</strong><br />
  Nested folders, vault, search, import/export, optional OIDC, and same-instance sharing.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-8458B3?style=flat-square" alt="MIT License" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/backend-Rust%20%2B%20axum-dea584?style=flat-square&logo=rust&logoColor=white" alt="Rust" /></a>
  <a href="https://react.dev/"><img src="https://img.shields.io/badge/frontend-React%20%2B%20Vite-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React" /></a>
  <a href="https://www.postgresql.org/"><img src="https://img.shields.io/badge/db-Postgres-4169E1?style=flat-square&logo=postgresql&logoColor=white" alt="Postgres" /></a>
  <a href="https://www.meilisearch.com/"><img src="https://img.shields.io/badge/search-Meilisearch-FF5CAA?style=flat-square" alt="Meilisearch" /></a>
  <a href="deploy/docker/docker-compose.yml"><img src="https://img.shields.io/badge/deploy-Docker%20Compose-2496ED?style=flat-square&logo=docker&logoColor=white" alt="Docker Compose" /></a>
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#configuration">Config</a> ·
  <a href="#development">Dev</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#docs">Docs</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#license">License</a>
</p>

<details>
<summary><strong>AI-assisted development</strong></summary>

<br />

Development used AI tooling under staff-engineer supervision (~20 years experience, 13 commercial). Architecture, logic, CI/CD, tests, and coverage were reviewed by the maintainer. AI accelerated implementation; technical decisions and review remained human-owned.

</details>

---

## Overview

Linkdean is a small, self-hosted bookmarks server: Postgres, optional Meilisearch, Docker Compose, env-based secrets, no proprietary control plane.

- **Data locality:** Postgres and optional Meilisearch on the operator's infrastructure
- **Organization:** nested categories, tags, hidden folders, grid and explorer views
- **Vault:** passphrase-protected categories (Argon2id + AES-GCM)
- **Import / export:** Netscape HTML, Pocket CSV, Raindrop CSV, Linkdean JSON
- **Auth:** local accounts and optional OIDC
- **Sharing:** invite same-instance users to a folder (read-only or read-write)

---

## Features

| Area | Capability |
| --- | --- |
| **Library** | Nested categories, tags, grid + explorer, bulk edit, flags, trash |
| **Dashboards** | Multi-board glance home; shareable layouts |
| **Search** | Meilisearch full-text for non-vault bookmarks |
| **Vault** | Passphrase-protected categories; plaintext never indexed |
| **Sharing** | Same-instance folder / dashboard ACL; invite and notification emails; likes, comments, ratings |
| **Auth** | Local password, password reset, email verification, optional OIDC with account linking, admin accounts, API keys |
| **Import / export** | Netscape, JSON, Pocket, Raindrop |
| **Ops** | Compose healthchecks, ready probe, backup recipe, admin reindex, optional SMTP email |

Full list: [docs/FEATURES.md](docs/FEATURES.md).

---

## Quick start

### Released image (recommended)

Run a published image from the GitHub Container Registry with a Compose file you write yourself, no checkout or build needed. [docs/INSTALL.md](docs/INSTALL.md) has the Compose file, optional Chrome screenshots, and every environment variable: which are required and what each does.

### Build from source with Docker Compose

```bash
git clone https://github.com/arsamsarabi/linkdean.git
cd linkdean
printf 'POSTGRES_PASSWORD=%s\nMEILI_MASTER_KEY=%s\nSESSION_SECRET=%s\n' \
  "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > deploy/docker/.env

docker compose --env-file deploy/docker/.env -f deploy/docker/docker-compose.yml up -d --build
```

Open **http://localhost:8080**. The first registered user becomes admin. On a server others can reach before you, set `BOOTSTRAP_TOKEN` so only you can create that account.

Production Compose refuses to start without `POSTGRES_PASSWORD`, `MEILI_MASTER_KEY` and `SESSION_SECRET`, and the API refuses the default values published in this repo. Behind a reverse proxy, also set `TRUST_FORWARDED_FOR=true`. Required secrets, proxy setup, limits and upgrade steps: [docs/SELF_HOSTED.md](docs/SELF_HOSTED.md#production-deployment).

Probes: [`/api/health`](http://localhost:8080/api/health) (liveness) · [`/api/ready`](http://localhost:8080/api/ready) (Postgres + Meili). Backup recipe in [docs/SELF_HOSTED.md](docs/SELF_HOSTED.md).

### Stack services

| Service | Role | Default port |
| --- | --- | --- |
| `api` | Linkdean API + static SPA | `8080` |
| `db` | Postgres 16 | Internal only (dev: host `5433`) |
| `meilisearch` | Search index | Internal only (dev: `7700`) |
| `chrome` | Optional CDP for page previews. Sandboxed, non-root, off the data network | Internal only (dev: `127.0.0.1:9222`) |
| `mailpit` | Dev only: catches outgoing email | Dev: `127.0.0.1:8025` |

See [docs/SELF_HOSTED.md](docs/SELF_HOSTED.md).

---

## Configuration

Runtime configuration is environment variables only. The full reference, with defaults and required ones marked, is in [docs/INSTALL.md](docs/INSTALL.md#environment-variables); local development uses [`.env.example`](.env.example).

| Variable | Required | Description |
| --- | --- | --- |
| `DATABASE_URL` | yes | Postgres connection string |
| `SESSION_SECRET` | yes | Long random secret. Published defaults are refused |
| `POSTGRES_PASSWORD` | Compose | Database password, used by the `db` and `api` services. The `linkdean:linkdean` login is refused |
| `ALLOW_DEFAULT_SECRETS` | | Development only. Without it, the API refuses to start on a well-known secret. Never set in production |
| `APP_URL` | yes | Public base URL (e.g. `http://localhost:8080`) |
| `BIND` | | Listen address (default `0.0.0.0:8080`) |
| `MEILI_URL` / `MEILI_MASTER_KEY` | Compose (key) | Meilisearch endpoint + key. Published default key is refused |
| `STATIC_DIR` | | Built SPA directory (Compose sets this) |
| `AVATAR_DIR` / `PREVIEW_DIR` | | Avatar uploads; screenshots + favicon/OG cache |
| `SESSION_DAYS` | | Session lifetime (default `30`) |
| `CORS_ORIGINS` | | Comma-separated origins; default = `APP_URL` |
| `TRUST_FORWARDED_FOR` | Behind a proxy | Set `true` behind a reverse proxy, else all clients share one address for login limits. Leave unset when the API is reachable directly |
| `BROWSER_WS_URL` | | Chrome CDP URL for screenshots |
| `BOOTSTRAP_TOKEN` | | When set, the first account (the admin) must enter it on the setup page, and that account must use a password, not OIDC. Unset, whoever registers first becomes admin. Ignored once any account exists |
| `SMTP_HOST` | | Mail server. Mail is on when this and `MAIL_FROM` are set; unset, nothing is emailed. See [Email](docs/SELF_HOSTED.md#email) |
| `SMTP_PORT` | | Default `587`, or `465` with `SMTP_TLS=tls` |
| `SMTP_USERNAME` / `SMTP_PASSWORD` | | Set both or neither |
| `SMTP_TLS` | | `starttls` (default), `tls`, or `none` (local servers only) |
| `MAIL_FROM` | | Sender, e.g. `Linkdean <no-reply@example.com>` |
| `METRICS_TOKEN` | | Turns on `GET /api/metrics`, which then needs `Authorization: Bearer <token>`. Unset, the route returns `404` |
| `OIDC_ISSUER` / `OIDC_CLIENT_ID` / `OIDC_CLIENT_SECRET` | | Optional OIDC. `OIDC_ISSUER` must match the issuer in the provider's discovery document exactly, trailing slash included. New identities get an account only while registration is open |

**OIDC callback:** `{APP_URL}/api/auth/oidc/callback`

---

## Development

### With live reload (recommended)

```bash
docker compose -f deploy/docker/docker-compose.dev.yml up --build
```

Open **http://localhost:5173**. The dev stack uses well-known secrets and sets `ALLOW_DEFAULT_SECRETS=true`, so it needs no `.env`. Its ports bind to `127.0.0.1` only, so other machines on the network cannot reach them. Web changes appear through Vite hot reload, and the API rebuilds and restarts when `crates/` or `migrations/` change. The first run compiles the API, so it takes a few minutes. `docker-compose.dev.yml` shares its project name and data volumes with `docker-compose.yml`, so run one at a time.

### On the host

**Prerequisites:** Rust (edition 2021), Node 20+, `pnpm`, Docker (Postgres / Meili).

```bash
# Infra only
docker compose -f deploy/docker/docker-compose.dev.yml up -d db meilisearch

cp .env.example .env   # keeps ALLOW_DEFAULT_SECRETS=true for the dev secrets
# Host → Compose Postgres is mapped to 5433:
# DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:5433/linkdean

# API (runs migrations on boot)
cargo run -p linkdean-api

# Web (Vite proxies /api → :8080)
cd apps/web && pnpm install && pnpm dev
```

Checks:

```bash
cargo test --workspace
cargo test -p linkdean-db --test authz   # needs DATABASE_URL
cd apps/web && pnpm lint:all && pnpm test
```

---

## Architecture

```text
Browser ──► Vite (dev) or static SPA
              │
              ▼ /api
         linkdean-api (axum)
              ├── Postgres
              ├── Meilisearch
              └── worker tasks (title / favicon / preview)
```

| Layer | Crate / app |
| --- | --- |
| HTTP | `crates/api` |
| Domain / crypto / import | `crates/core` |
| SQL + migrations | `crates/db` + `migrations/` |
| Search | `crates/search` |
| Email (templates + SMTP) | `crates/mail` |
| Enrichment | `crates/worker` |
| SPA | `apps/web` |

Full write-up: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Security notes

- Sessions are opaque tokens; mutating routes require auth
- Owner and share ACL filter every category / bookmark query; client-supplied owner ids are ignored
- Vault payloads encrypted at rest; vault content is **not** sent to Meilisearch
- Export never dumps vault bookmarks as plaintext
- Import files should be treated as untrusted input

Vulnerability reports: prefer a private report to the maintainers before a public issue.

---

## Docs

| Doc | Contents |
| --- | --- |
| [Features](docs/FEATURES.md) | What the app can do today |
| [Architecture](docs/ARCHITECTURE.md) | Stack, domain model, auth, vault vs hidden |
| [Roadmap](docs/ROADMAP.md) | Planned milestones / backlog |
| [Self-host](docs/SELF_HOSTED.md) | Production secrets, reverse proxy, limits, upgrades, backup |
| [Brand guidelines](brand/GUIDELINES.md) | Logo, colour, clear space |

---

## Roadmap

**Next up:** browser extension (save tab / file to category). Backlog includes search filters, notification digests, PWA, and more. Far out: native apps, archive/downloads, gamification.

- Capabilities: [docs/FEATURES.md](docs/FEATURES.md)
- Plan: [docs/ROADMAP.md](docs/ROADMAP.md)

---

## Contributing

Pull requests should stay focused:

1. Follow crate / frontend playbooks (`crates/AGENTS.md`, `apps/web/AGENTS.md`)
2. Preserve authz and vault invariants; extend tests when changing access paths
3. Schema changes go through `migrations/` only
4. Prefer small, reviewable PRs

---

## License

[MIT](LICENSE) © Linkdean Contributors

<p align="center">
  <img src="brand/exports/png/linkdean-symbol-color-256.png" alt="Linkdean symbol" width="64" />
  <br />
  <sub>Your corner of the internet, organised.</sub>
</p>
