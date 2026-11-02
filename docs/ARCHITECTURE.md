# Architecture

Linkdean is a self-hostable bookmarks manager. Configuration uses environment variables only (no Infisical SDK).

## Stack

| Layer | Choice |
| --- | --- |
| API | Rust · axum · sqlx |
| DB | Postgres |
| Search | Meilisearch (non-vault) |
| Web | Vite · React · TS · TanStack Router/Query · Mantine · Zustand |
| Secrets | `.env` / compose env |
| Auth | Local password + optional OIDC |

## Repo layout

```
linkdean/
  apps/web/           # SPA
  crates/api/         # HTTP binary
  crates/core/        # domain, crypto, import/export
  crates/db/          # Postgres repos + migrations
  crates/search/      # Meilisearch client
  crates/worker/      # metadata + favicon/OG disk cache
  migrations/         # sqlx migrations
  deploy/docker/      # compose + Dockerfile
  docs/
```

## Runtime shape

```text
Browser ──► Vite (dev) or static SPA
              │
              ▼ /api
         linkdean-api (axum)
              ├── Postgres
              ├── Meilisearch
              └── worker tasks (enrich title/favicon)
```

## Domain (v1)

- **User:** local and/or OIDC; first user = `is_admin`; `onboarding_completed`; profile fields + avatar
- **UserSettings:** per-user prefs (hidden, vault search, theme, auto-lock, landing)
- **Category:** tree (`parent_id`); flags `is_hidden`, `is_vault`; `vault_id` when inside a vault
- **Tag:** per-user; M2M with bookmarks
- **Bookmark:** url/title/notes/tags; vault rows store ciphertext instead of plaintext fields
- **Vault:** named, many per user; data key wrapped by its passphrase, by its recovery key, and optionally by the user's master key for the global passphrase (Argon2id + AES-GCM). Unlocked keys live in API memory per login session (`crates/api/src/vault_keys.rs`), keyed by the session token hash. API keys never get vault keys. Keys are dropped after the user's auto-lock time with no request using them, on logout or password change, and when the hourly maintenance task finds the session gone
- **ImportJob:** import run stats

Rows carry `owner_user_id`. Milestone 1 adds `category_shares` (invitee + `read`/`write`) beside ownership; access covers the shared category and its non-vault descendants. Milestone 2 adds `dashboards` / `dashboard_sections` / `dashboard_items` for a per-user home layout (default dashboard auto-created). Categories may also carry an unlisted `public_slug` for anonymous read-only list URLs (`GET /api/public/lists/{slug}` → SPA `/l/{slug}`).

## Auth & sessions

- Register / login issues an opaque session token (cookie + `Authorization: Bearer`)
- Expired or revoked session → `401 {"code":"session_expired"}`; the SPA clears its token and opens `/login`. A wrong password stays a plain `401`
- In-memory rate limits (`crates/api/src/rate_limit.rs`) on login, registration and vault unlock/recovery → `429`. Counters reset on restart. Client address comes from the TCP peer, or from the last `X-Forwarded-For` entry when `TRUST_FORWARDED_FOR=true`
- Argon2 work runs on a blocking pool, at most 4 at once (`crates/db/src/repo/blocking.rs`)
- Startup refuses published secrets unless `ALLOW_DEFAULT_SECRETS=true` (dev only)
- First install uses `/onboarding` (admin account, optional vault, optional import)
- OIDC: Authorization Code with discovery, PKCE, state and nonce against `OIDC_*` env; callback `/api/auth/oidc/callback`. State, nonce and verifier ride in a 10 minute HttpOnly `linkdean_oidc` cookie scoped to `/api/auth/oidc`. The ID token is validated (signature, issuer, audience, expiry, nonce); userinfo fills in a missing email. Users are keyed by issuer and subject, never by email. New identities obey closed registration. The callback always redirects: home with a session cookie, or `/login?oidc_error=…`
- Profile (`/profile`) and settings (`/settings`); header avatar menu
- Avatars: client crop, server resize to 256×256 WebP under `AVATAR_DIR`
- Prefs in `user_settings` (show hidden, vault search mode, theme, auto-lock, landing `/`|`/library`)

## API conventions

- `PATCH` bodies: an omitted field stays unchanged; `null` clears it (profile name/bio/timezone, bookmark description/notes/category, category parent/icon)
- Per-route body limits: import 20 MiB (10 MiB file content), avatar 6 MiB, preview 9 MiB
- Each Postgres statement times out after 30 s; migrations run on a separate connection without the limit
- `GET /api/metrics` needs `Authorization: Bearer $METRICS_TOKEN` and is off when the token is unset. Labels use the route template, not the raw path. `GET /api/ready` returns booleans only
- Hourly maintenance (`crates/api/src/maintenance.rs`): trash retention, with search and preview cleanup for purged bookmarks, the expired-session purge, and removal of unused tags more than an hour old

Operator-facing limits and behaviour: [self-host.md](./self-host.md#limits-and-behaviour-operators-should-know).

## Hidden vs vault

| | Hidden | Vault |
| --- | --- | --- |
| Purpose | Declutter UI | Sensitive bookmarks |
| Storage | Plaintext | Encrypted at rest |
| Search | Meilisearch | Only while unlocked (Postgres scan) |
| Access | Show-hidden toggle | Passphrase unlock (server-held key for session) |

## Search

Writes upsert non-vault docs into Meilisearch index `bookmarks` using the bookmark's `owner_user_id` and `category_ids`. Queries filter by owner or readable categories (owned + shared). Vault docs stay out of that index.

## Import / export

| Format | Direction |
| --- | --- |
| Netscape HTML | in / out |
| Linkdean JSON | in / out |
| Pocket CSV | in |
| Raindrop CSV | in |

Vault bookmarks are not exported as plaintext.

Linkdean JSON is versioned (`EXPORT_VERSION` in `crates/core/src/import_export/mod.rs`, now 2). Version 1 files still import. A file newer than the server is refused. Trash and vault content are left out.

## Local dev

`deploy/docker/docker-compose.dev.yml` runs everything with live reload on `:5173`. `deploy/docker/docker-compose.yml` is the production stack and needs `POSTGRES_PASSWORD`, `MEILI_MASTER_KEY` and `SESSION_SECRET` (see [self-host.md](./self-host.md#production-deployment)). The dev stack uses published secrets with `ALLOW_DEFAULT_SECRETS=true`. Both use the same project name and volumes, so run one at a time.

```bash
docker compose -f deploy/docker/docker-compose.dev.yml up --build
```

To run the API and web on the host instead:

```bash
docker compose -f deploy/docker/docker-compose.dev.yml up -d db meilisearch
cp .env.example .env   # DATABASE_URL → :5433; keeps ALLOW_DEFAULT_SECRETS=true
cargo run -p linkdean-api
cd apps/web && pnpm dev   # :5173 proxies /api → :8080
```

See also [FEATURES.md](./FEATURES.md), [self-host.md](./self-host.md), and [ROADMAP.md](./ROADMAP.md).
