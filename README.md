<p align="center">
  <img src="brand/graphics/the-link-dean.png" alt="The Link Dean — Linkdean mascot" width="420" />
</p>

<h1 align="center">Linkdean</h1>

<p align="center">
  <strong>Self-hostable bookmarks for people who actually keep them.</strong><br />
  Nested folders · vault · search · import/export · optional OIDC · sharing on your instance.
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
<summary><strong>Click here to find out how AI was used in the development of this app</strong></summary>

<br />

I built this with the help of AI, but with hands-on supervision throughout the process. I’m a Staff Software Engineer with ~20 years of coding experience (13 years commercially), and I carefully reviewed the architecture, logic, CI/CD pipelines, tests, and code coverage. AI helped speed things up, but the technical decisions and reviews were mine.

</details>

---

## Why Linkdean?

Browser bookmark bars rot. SaaS bookmark apps lock you in. **Linkdean** is a small, opinionated bookmarks server you run yourself — homelab-friendly, env-only secrets, no proprietary control plane.

- **Your data, your box** — Postgres + optional Meilisearch, Docker Compose in minutes
- **Organize for real** — nested categories, tags, hidden folders, grid + explorer views
- **Sensitive links stay sensitive** — password-protected vault categories (Argon2id + AES-GCM)
- **Bring your library** — Netscape HTML, Pocket CSV, Raindrop CSV, Linkdean JSON
- **Sign in your way** — local accounts and/or OIDC (Authentik and friends)
- **Collaborate** — share folders with other users on the same instance (read-only / read-write)

---

## Features

| Area | What you get |
| --- | --- |
| **Library** | Nested categories, multi-category bookmarks, tags, grid + explorer layouts |
| **Search** | Meilisearch full-text for non-vault bookmarks |
| **Vault** | Passphrase-protected categories; plaintext never indexed |
| **Hidden** | Declutter without encrypting |
| **Import / export** | Netscape HTML, Linkdean JSON, Pocket & Raindrop CSV in; JSON + Netscape out |
| **Auth** | Local password, optional OIDC, first-user admin onboarding, open-registration toggle |
| **Sharing** | Invite same-instance users to a folder as **read** or **write** |
| **Enrichment** | Title / favicon / preview fetch (optional headless Chrome) |
| **Ops** | Single Compose stack, avatars + previews on volumes, health endpoint |

---

## Quick start

### Docker Compose (recommended)

```bash
git clone https://github.com/arsamsarabi/linkdean.git
cd linkdean
cp .env.example .env
# set SESSION_SECRET to a long random string

cd deploy/docker
docker compose up -d --build
```

Open **http://localhost:8080** → register the first user (becomes admin).

### What the stack runs

| Service | Role | Default port |
| --- | --- | --- |
| `api` | Linkdean API + static SPA | `8080` |
| `db` | Postgres 16 | host `5433` → container `5432` |
| `meilisearch` | Search index | `7700` |
| `chrome` | Optional CDP for page previews | `9222` |

More detail: [docs/self-host.md](docs/self-host.md).

---

## Configuration

All runtime config is **environment variables** (see [`.env.example`](.env.example)). No SaaS secret manager required.

| Variable | Required | Description |
| --- | --- | --- |
| `DATABASE_URL` | ✅ | Postgres connection string |
| `SESSION_SECRET` | ✅ | Long random secret (reserved / cookie signing) |
| `APP_URL` | ✅ | Public base URL (e.g. `http://localhost:8080`) |
| `BIND` | | Listen address (default `0.0.0.0:8080`) |
| `MEILI_URL` / `MEILI_MASTER_KEY` | | Meilisearch endpoint + key |
| `STATIC_DIR` | | Built SPA directory (Compose sets this) |
| `AVATAR_DIR` / `PREVIEW_DIR` | | Upload / preview storage paths |
| `SESSION_DAYS` | | Session lifetime (default `30`) |
| `CORS_ORIGINS` | | Comma-separated origins; default = `APP_URL` |
| `BROWSER_WS_URL` | | Chrome CDP URL for screenshots |
| `OIDC_ISSUER` / `OIDC_CLIENT_ID` / `OIDC_CLIENT_SECRET` | | Optional OIDC |

**OIDC callback:** `{APP_URL}/api/auth/oidc/callback`

---

## Development

**Prereqs:** Rust (edition 2021), Node 20+, `pnpm`, Docker (for Postgres / Meili).

```bash
# Infra only
docker compose -f deploy/docker/docker-compose.yml up -d db meilisearch

cp .env.example .env
# Host → Compose Postgres is mapped to 5433:
# DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:5433/linkdean

# API (runs migrations on boot)
cargo run -p linkdean-api

# Web (Vite proxies /api → :8080)
cd apps/web && pnpm install && pnpm dev
```

Useful checks:

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
| Enrichment | `crates/worker` |
| SPA | `apps/web` |

Full write-up: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## Security notes

- Sessions are opaque tokens; mutating routes require auth
- Owner (and share ACL) filters every category / bookmark query — no client-supplied owner ids
- Vault payloads encrypted at rest; vault content is **not** sent to Meilisearch
- Export never dumps vault bookmarks as plaintext
- Treat import files as hostile input

Found a vuln? Prefer a private report to the maintainers before a public issue.

---

## Docs

| Doc | Contents |
| --- | --- |
| [Architecture](docs/ARCHITECTURE.md) | Stack, domain model, auth, vault vs hidden |
| [Roadmap](docs/ROADMAP.md) | Build order / milestones |
| [Self-host](docs/self-host.md) | Compose + OIDC checklist |
| [Brand guidelines](brand/GUIDELINES.md) | Logo, colour, clear space |

---

## Roadmap

**Shipped (v1):** nested cats + tags, Meili search, import/export, vault, hidden folders, local + OIDC auth, Compose path, library views, admin registration toggle.

**Next (v2+):** sharing polish → personal dashboards → shared dashboards → API keys → archive/downloads → browser extension.

See the full ordered list in [docs/ROADMAP.md](docs/ROADMAP.md).

---

## Contributing

PRs welcome. Keep changes focused:

1. Match crate / frontend playbooks (`crates/AGENTS.md`, `apps/web/AGENTS.md`)
2. Authz and vault invariants are non-negotiable — extend tests when touching access paths
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
