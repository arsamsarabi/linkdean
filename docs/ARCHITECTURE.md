# Architecture

Linkdean is a self-hostable bookmarks manager. Homelab-friendly, OSS-portable (env vars only — no Infisical SDK).

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
  crates/worker/      # metadata/favicon fetch
  migrations/         # sqlx migrations
  deploy/docker/      # compose + Dockerfile
  deploy/homelab/     # Traefik snippet + notes
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

Homelab: nova runs compose; ncc Traefik terminates TLS (`linkdean.arsam.app`). Authik OIDC optional.

## Domain (v1)

- **User** — local and/or OIDC; first user = `is_admin`; `onboarding_completed`; profile fields + avatar
- **UserSettings** — per-user prefs (hidden, vault search, theme, auto-lock, landing)
- **Category** — tree (`parent_id`); flags `is_hidden`, `is_vault`
- **Tag** — per-user; M2M with bookmarks
- **Bookmark** — url/title/notes/tags; vault rows store ciphertext instead of plaintext fields
- **Vault** — per-user wrapped data key (Argon2id + AES-GCM)
- **ImportJob** — import run stats

Rows carry `owner_user_id` now so v2 sharing can land without a rewrite.

## Auth & sessions

- Register / login → opaque session token (cookie + `Authorization: Bearer`)
- First install → `/onboarding` wizard (admin account → optional vault → optional import)
- OIDC: Authorization Code against `OIDC_*` env; callback `/api/auth/oidc/callback`
- Profile (`/profile`) + settings (`/settings`); header avatar menu
- Avatars: crop client-side → server resizes to 256×256 WebP under `AVATAR_DIR`
- Prefs in `user_settings` (show hidden, vault search mode, theme, auto-lock, landing)

## Hidden vs vault

| | Hidden | Vault |
| --- | --- | --- |
| Purpose | Declutter UI | Sensitive bookmarks |
| Storage | Plaintext | Encrypted at rest |
| Search | Meilisearch | Only while unlocked (Postgres scan) |
| Access | “Show hidden” toggle | Passphrase unlock (server-held key for session) |

## Search

Writes upsert non-vault docs into Meilisearch index `bookmarks`, filtered by `owner_user_id`. Vault docs stay out of that index.

## Import / export

| Format | Direction |
| --- | --- |
| Netscape HTML | in / out |
| Linkdean JSON | in / out |
| Pocket CSV | in |
| Raindrop CSV | in |

Vault bookmarks are not exported as plaintext.

## Local dev

```bash
docker compose -f deploy/docker/docker-compose.yml up -d db meilisearch
cp .env.example .env   # SESSION_SECRET, DATABASE_URL → :5433
cargo run -p linkdean-api
cd apps/web && pnpm dev   # :5173 proxies /api → :8080
```

See also [self-host.md](./self-host.md) and [ROADMAP.md](./ROADMAP.md).
