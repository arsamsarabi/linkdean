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
- **Vault:** named, many per user; data key wrapped by its passphrase, by its recovery key, and optionally by the user's master key for the global passphrase (Argon2id + AES-GCM). Unlocked keys live in API memory, per user and vault
- **ImportJob:** import run stats

Rows carry `owner_user_id`. Milestone 1 adds `category_shares` (invitee + `read`/`write`) beside ownership; access covers the shared category and its non-vault descendants. Milestone 2 adds `dashboards` / `dashboard_sections` / `dashboard_items` for a per-user home layout (default dashboard auto-created). Categories may also carry an unlisted `public_slug` for anonymous read-only list URLs (`GET /api/public/lists/{slug}` → SPA `/l/{slug}`).

## Auth & sessions

- Register / login issues an opaque session token (cookie + `Authorization: Bearer`)
- First install uses `/onboarding` (admin account, optional vault, optional import)
- OIDC: Authorization Code against `OIDC_*` env; callback `/api/auth/oidc/callback`
- Profile (`/profile`) and settings (`/settings`); header avatar menu
- Avatars: client crop, server resize to 256×256 WebP under `AVATAR_DIR`
- Prefs in `user_settings` (show hidden, vault search mode, theme, auto-lock, landing `/`|`/library`)

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

## Local dev

`deploy/docker/docker-compose.dev.yml` runs everything with live reload on `:5173`. `deploy/docker/docker-compose.yml` is the production stack. Both use the same project name and volumes, so run one at a time.

```bash
docker compose -f deploy/docker/docker-compose.dev.yml up --build
```

To run the API and web on the host instead:

```bash
docker compose -f deploy/docker/docker-compose.dev.yml up -d db meilisearch
cp .env.example .env   # SESSION_SECRET, DATABASE_URL → :5433
cargo run -p linkdean-api
cd apps/web && pnpm dev   # :5173 proxies /api → :8080
```

See also [FEATURES.md](./FEATURES.md), [self-host.md](./self-host.md), and [ROADMAP.md](./ROADMAP.md).
