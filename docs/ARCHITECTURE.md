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
  crates/mail/        # email config, templates, SMTP
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
              ├── SMTP (optional, email outbox sender)
              └── worker tasks (enrich title/favicon)
```

## Domain (v1)

- **User:** local and/or OIDC; first user = `is_admin`; `onboarding_completed`; profile fields + avatar
- **UserSettings:** per-user prefs (hidden, vault search, theme, auto-lock, landing)
- **Category:** tree (`parent_id`); flags `is_hidden`, `is_vault`; `vault_id` when inside a vault
- **Tag:** per-user; M2M with bookmarks
- **Bookmark:** url/title/notes/tags; vault rows store ciphertext instead of plaintext fields
- **Vault:** named, many per user; data key wrapped by its passphrase, by its recovery key, and optionally by the user's master key for the global passphrase (Argon2id + AES-GCM). Each wrap's salt records the Argon2 cost it used, so the cost can rise without breaking older wraps. Password hashes carry their cost in the PHC string and are rehashed at sign-in when it was lower. Unlocked keys live in API memory per login session (`crates/api/src/vault_keys.rs`), keyed by the session token hash. API keys never get vault keys. Keys are dropped after the user's auto-lock time with no request using them, on logout or password change, and when the hourly maintenance task finds the session gone. Replacing a recovery key and turning off the global passphrase take the passphrase again, even while unlocked, and trashed vault content is purged only while its vault is unlocked
- **ImportJob:** import run stats

Rows carry `owner_user_id`. Milestone 1 adds `category_shares` (invitee + `read`/`write`) beside ownership; access covers the shared category and its non-vault descendants. Milestone 2 adds `dashboards` / `dashboard_sections` / `dashboard_items` for a per-user home layout (default dashboard auto-created). Categories may also carry an unlisted `public_slug` for anonymous read-only list URLs (`GET /api/public/lists/{slug}` → SPA `/l/{slug}`).

## Auth & sessions

- Register / login issues an opaque session token: an `HttpOnly`, `SameSite=Lax` `linkdean_session` cookie, and the token in the response body for API clients. The web app keeps no token: it sends the cookie and an `X-Linkdean-Client: web` header, and stores only a non-secret signed-in marker. A cookie authorises a write (any method but GET, HEAD, OPTIONS) only with that header, which a cross-site page cannot add without a CORS preflight the API never grants with credentials. `Authorization: Bearer` (sessions and API keys) needs no header
- Expired or revoked session → `401 {"code":"session_expired"}`; the SPA clears its signed-in marker and opens `/login`. A wrong password stays a plain `401`
- In-memory rate limits (`crates/api/src/rate_limit.rs`) on login, registration and vault passphrase checks (unlock, recovery, recovery key replacement, global unlock and turning the global passphrase off) → `429`. Counters reset on restart. Client address comes from the TCP peer, or from the last `X-Forwarded-For` entry when `TRUST_FORWARDED_FOR=true`
- Argon2 work runs on a blocking pool, at most 4 at once (`crates/db/src/repo/blocking.rs`)
- Startup refuses published secrets unless `ALLOW_DEFAULT_SECRETS=true` (dev only)
- First install uses `/onboarding` (admin account, optional vault, optional import). Sign-ups run under the `user_signup` advisory lock in one transaction, so only one account can become the first admin and a registration invite is claimed (`FOR UPDATE`) with the user insert. With `BOOTSTRAP_TOKEN` set, the first account must present it; OIDC cannot, so it is refused until an admin exists
- OIDC: Authorization Code with discovery, PKCE, state and nonce against `OIDC_*` env; callback `/api/auth/oidc/callback`. State, nonce and verifier ride in a 10 minute HttpOnly `linkdean_oidc` cookie scoped to `/api/auth/oidc`. The ID token is validated (signature, issuer, audience, expiry, nonce); userinfo fills in a missing email. Users are keyed by issuer and subject, never by email. New identities obey closed registration. The callback always redirects: home with a session cookie, or `/login?oidc_error=…`
- Profile (`/profile`) and settings (`/settings`); header avatar menu
- Avatars: client crop, server resize to 256×256 WebP under `AVATAR_DIR`
- Prefs in `user_settings` (show hidden, vault search mode, theme, auto-lock, landing `/`|`/library`)

## API conventions

- `PATCH` bodies: an omitted field stays unchanged; `null` clears it (profile name/bio/timezone, bookmark description/notes/category, category parent/icon). Bookmark `add_tags` and `add_category_ids` add to what the bookmark holds instead of replacing it
- Per-route body limits: import 20 MiB (10 MiB file content), avatar 6 MiB, preview 9 MiB. Uploaded images over 40 megapixels are refused from their header, each decode may allocate at most 160 MiB, and two decode at once.
- Each Postgres statement times out after 30 s; migrations run on a separate connection without the limit
- `GET /api/metrics` needs `Authorization: Bearer $METRICS_TOKEN` and is off when the token is unset. Labels use the route template, not the raw path. `GET /api/ready` returns booleans only
- Hourly maintenance (`crates/api/src/maintenance.rs`): trash retention, with search and preview cleanup for purged bookmarks, the expired-session purge, removal of unused tags more than an hour old, and the email outbox purge

Operator-facing limits and behaviour: [SELF_HOSTED.md](./SELF_HOSTED.md#limits-and-behaviour-operators-should-know).

## Email

- `crates/mail` owns config, rendering and delivery. `MailConfig::from_vars` reads `SMTP_*` and `MAIL_FROM`; mail is off unless `SMTP_HOST` and `MAIL_FROM` are both set, and half or unsafe settings stop startup (`SMTP_TLS=none` only for local hosts)
- Templates live in `crates/mail/templates/` and are embedded at compile time: `<name>.mjml`, `<name>.subject` and `<name>.sample.json`, sharing `layout.mjml` and `macros.mjml`. minijinja (strict undefined, HTML autoescape) fills them, `mrml` turns the MJML into HTML, and `html2text` makes the plain-text part. Every template gets `site.app_url` and `site.instance`
- `lettre` sends over SMTP with a 20 s timeout per step, a `Message-ID` on every message, and HTML plus plain-text alternatives
- `email_outbox` (`migrations/040_email_outbox.sql`) stores rendered emails, so a later template change never alters a queued one. Handlers call `mail::enqueue` (`crates/api/src/mail.rs`), which returns `Disabled` when mail is off and never sends inline
- The API runs a sender loop next to maintenance: every 5 s it claims up to 20 due emails under a five-minute lease (`FOR UPDATE SKIP LOCKED`), renews the lease before each send, and gives each send 60 s. A failure retries after 30 s, doubling to at most 1 h; an email still unsent 24 h after it was queued is marked failed. A crashed sender's claims come back when the lease expires
- Delivery is at least once: if the process stops after the SMTP server accepted an email but before it is marked sent, it is sent again once the lease expires
- Hourly maintenance deletes sent rows after 7 days and failed rows after 30
- `GET /api/admin/email` (status, settings without credentials, 24 h counts, latest error) and `POST /api/admin/email/test` (direct send to the admin, 5 per admin per 15 minutes), both admin only
- `/api/dev/emails` lists templates and `/api/dev/emails/{name}` renders one with its sample data (`?format=text` for the text part). Mounted only when `ALLOW_DEFAULT_SECRETS=true`. Dev Compose runs Mailpit, which catches all dev email (web UI on `127.0.0.1:8025`)

## Hidden vs vault

| | Hidden | Vault |
| --- | --- | --- |
| Purpose | Declutter UI | Sensitive bookmarks |
| Storage | Plaintext | Encrypted at rest |
| Search | Meilisearch | Only while unlocked (Postgres scan) |
| Access | Show-hidden toggle | Passphrase unlock (server-held key for session) |

## Search

Writes upsert non-vault docs into Meilisearch index `bookmarks` using the bookmark's `owner_user_id` and `category_ids`. Queries filter by owner or readable categories (owned + shared). Vault docs stay out of that index. Each hit is rechecked against Postgres before it is returned: vault, trashed or unreadable bookmarks are dropped, and stale documents are deleted from the index.

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

`deploy/docker/docker-compose.dev.yml` runs everything with live reload on `:5173`. `deploy/docker/docker-compose.yml` is the production stack and needs `POSTGRES_PASSWORD`, `MEILI_MASTER_KEY` and `SESSION_SECRET` (see [SELF_HOSTED.md](./SELF_HOSTED.md#production-deployment)). The dev stack uses published secrets with `ALLOW_DEFAULT_SECRETS=true`. Both use the same project name and volumes, so run one at a time.

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

See also [FEATURES.md](./FEATURES.md), [SELF_HOSTED.md](./SELF_HOSTED.md), and [ROADMAP.md](./ROADMAP.md).
