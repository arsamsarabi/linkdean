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
- Expired or revoked session → `401 {"code":"session_expired"}`; the SPA clears its signed-in marker and opens `/login`, except on `/verify-email`, which shows its result with a Sign in button. A wrong password stays a plain `401`
- In-memory rate limits (`crates/api/src/rate_limit.rs`) on login, registration, password reset (requests: 3 per email and 10 per client address; link uses: 10 per client address; all per 15 minutes), email verification and email change (emails: 3 per account and 10 per client address; link uses: 10 per client address; all per 15 minutes) and vault passphrase checks (unlock, recovery, recovery key replacement, global unlock and turning the global passphrase off) → `429`. Counters reset on restart. Client address comes from the TCP peer, or from the last `X-Forwarded-For` entry when `TRUST_FORWARDED_FOR=true`
- Account tokens (`crates/db/src/repo/account_tokens.rs`, `migrations/042_email_verification.sql`): `account_tokens` stores the SHA-256 of a 32-byte random token, its `purpose` (`password_reset`, `verify_email`, `change_email`), its expiry, used time, the admin who made it, if any, and for `change_email` the target address. A token works only for its purpose and only once. Each user has at most one live token per purpose: a new one voids the previous one of that purpose and leaves the others. Every link puts the token in the URL fragment (`fragment_link`), so it never reaches server logs or a `Referer`, and the page clears it from the address bar. Hourly maintenance purges tokens a day after they were used or expired
- Password reset (`crates/api/src/routes/auth/password_reset.rs`): a `password_reset` account token. One live link per user: a new link (email or admin) voids the previous one, and a password change voids it too. `POST /api/auth/password/forgot` answers the same at once for every address and does the lookup and email in a background task, so neither the reply nor its timing says whether an account exists; it is refused when mail is off. Emailed links last 30 minutes; `POST /api/admin/users/{id}/reset-link` makes a 24-hour link (refused for SSO-only accounts) and returns it to the admin. Links open `/reset-password#token=…`: the token sits in the URL fragment, so it never reaches server logs or a `Referer`, and the page clears it from the address bar. `POST /api/auth/password/reset` uses the token once in one transaction (row lock, so two concurrent uses cannot both win), sets the password, ends every session, optionally revokes API keys, drops unlocked vault keys and signs the caller in with a new session
- Email verification (`crates/api/src/routes/auth/email_verification.rs`, `crates/db/src/repo/email_verification.rs`): `users.email_verified_at` records when the account proved it controls its address; migration `042` marks every account that existed then as verified. A password registration enqueues `verify_email` when mail is on (best effort; the registration never fails on it), and `POST /api/auth/email/resend` sends a new link (refused with mail off; answers `verified: true` when already verified). Links open `/verify-email#token=…` and last 24 hours (`EMAIL_VERIFY_LINK_HOURS`). `POST /api/auth/email/verify` needs no session: the token proves control of the inbox. OIDC sign-in marks the account verified when the ID token or userinfo reports `email_verified: true`, for new and existing accounts
- Invite routing: a folder or dashboard share to an address whose account is unverified is stored as an address-only pending invite (no `invitee_user_id`, no notification), the same as an address with no account. Verifying (link, admin or OIDC), confirming an email change and an admin email change call `link_pending_share_invites`, which attaches the waiting invites to the account; they still need accepting. Removing verification keeps accepted shares and routes only new invites to pending. `POST /api/shares/claim` lets a signed-in account claim an invite link sent to its current address (400 otherwise), which is how invites reach accounts with mail off
- Email change: `POST /api/auth/email/change` takes the current password and the new address (refused for SSO-only accounts, with mail off, and for an address another account uses). It makes a `change_email` token holding the new address, emails `email_change_verify` with `/verify-email#token=…&change=1` to the new address and `email_change_notice` (masked as `n•••@domain`) to the old one. The address changes only at `POST /api/auth/email/confirm-change`, which checks again that it is free (a unique violation is a `409`), switches it, marks it verified, voids every live account token and attaches invites waiting for the new address. `PATCH /api/users/me` no longer changes the email. Admins use `POST /api/admin/users/{id}/email` (applied at once, counts as verified) and `POST /api/admin/users/{id}/email-verified {verified}`
- Hard gate (`crates/api/src/auth/mod.rs`): with the `platform_settings.require_verified_email` setting on, the `AuthUser` extractor answers `403 {"code":"email_unverified"}` for an unverified, non-admin account on every path outside `/api/auth/*` (read from `OriginalUri`, since the nested router strips `/api`). Admins are never gated, so the setting cannot lock everyone out. Only unverified accounts trigger the platform settings lookup; verified ones pass without another query. The admin platform `PATCH` refuses to turn it on while mail is off, and the gate does not apply while mail is off even if the setting was left on. `/api/auth/me` reports the effective value, and the SPA sends gated accounts to the `/verify-email` screen (Resend, Sign out)
- Argon2 work runs on a blocking pool, at most 4 at once (`crates/db/src/repo/blocking.rs`)
- Startup refuses published secrets unless `ALLOW_DEFAULT_SECRETS=true` (dev only)
- First install uses `/onboarding` (admin account, optional vault, optional import). Sign-ups run under the `user_signup` advisory lock in one transaction, so only one account can become the first admin and a registration invite is claimed (`FOR UPDATE`) with the user insert. With `BOOTSTRAP_TOKEN` set, the first account must present it; OIDC cannot, so it is refused until an admin exists
- OIDC: Authorization Code with discovery, PKCE, state and nonce against `OIDC_*` env; callback `/api/auth/oidc/callback`. State, nonce and verifier ride in a 10 minute HttpOnly `linkdean_oidc` cookie scoped to `/api/auth/oidc`. The ID token is validated (signature, issuer, audience, expiry, nonce); userinfo fills in a missing email. Users are keyed by issuer and subject, never by email. New identities obey closed registration. The callback always redirects: home with a session cookie, `/login?link=1` with a pending link, or `/login?oidc_error=…`
- Single sign-on linking (`crates/db/src/repo/oidc_links.rs`, `routes/auth/oidc.rs`): an unknown identity whose address belongs to a password account (case-insensitive) creates a `pending_oidc_links` row (migration `043`: issuer, subject, provider email, provider `email_verified`, 10 minute expiry, `PENDING_OIDC_LINK_MINUTES`). Only the SHA-256 of its 32-byte token is stored; the token rides in an HttpOnly, SameSite=Lax `linkdean_oidc_link` cookie scoped to `/api/auth`. `GET /api/auth/oidc/pending-link` returns the masked provider address or 404. `POST /api/auth/login` with the cookie consumes the link in one transaction under a row lock: it attaches the identity only when the signed-in account's address equals the link's, marks the account verified only when the provider verified that address, adds `"sso_connected": true` to the response, enqueues `sso_connected`, and always expires the cookie; the link is spent either way and a failure never fails the sign-in. SSO-only holders and holders already linked to another identity are refused with `oidc_error`
- Connect and disconnect: `GET /api/auth/oidc/start?mode=connect` needs a session and records the user id in the flow cookie; the callback checks that the session still belongs to that user, attaches the identity whatever its address (`link_oidc_identity`), enqueues `sso_connected`, and redirects to `/profile?sso=connected` or `/profile?oidc_error=…`. `POST /api/auth/oidc/disconnect {password}` clears the identity after a password check (400 for accounts without a password, 401 for a wrong one, counted on the login limits). A unique index on `(oidc_issuer, oidc_subject)` keeps one identity per account and per subject; a taken identity is a 409 "This sign-in is already connected to another account." `UserPublic` carries `sso_connected` and `sso_provider` (issuer host). Maintenance purges links used or expired over a day ago
- Profile (`/profile`) and settings (`/settings`); header avatar menu
- Avatars: client crop, server resize to 256×256 WebP under `AVATAR_DIR`
- Prefs in `user_settings` (show hidden, vault search mode, theme, auto-lock, landing `/`|`/library`)

## API conventions

- `PATCH` bodies: an omitted field stays unchanged; `null` clears it (profile name/bio/timezone, bookmark description/notes/category, category parent/icon). Bookmark `add_tags` and `add_category_ids` add to what the bookmark holds instead of replacing it
- Per-route body limits: import 20 MiB (10 MiB file content), avatar 6 MiB, preview 9 MiB. Uploaded images over 40 megapixels are refused from their header, each decode may allocate at most 160 MiB, and two decode at once.
- Each Postgres statement times out after 30 s; migrations run on a separate connection without the limit
- `GET /api/metrics` needs `Authorization: Bearer $METRICS_TOKEN` and is off when the token is unset. Labels use the route template, not the raw path. `GET /api/ready` returns booleans only
- Hourly maintenance (`crates/api/src/maintenance.rs`): trash retention, with search and preview cleanup for purged bookmarks, the expired-session purge, removal of unused tags more than an hour old, the email outbox purge and the account token purge

Operator-facing limits and behaviour: [SELF_HOSTED.md](./SELF_HOSTED.md#limits-and-behaviour-operators-should-know).

## Email

- `crates/mail` owns config, rendering and delivery. `MailConfig::from_vars` reads `SMTP_*` and `MAIL_FROM`; mail is off unless `SMTP_HOST` and `MAIL_FROM` are both set, and half or unsafe settings stop startup (`SMTP_TLS=none` only for local hosts)
- Templates live in `crates/mail/templates/` and are embedded at compile time: `<name>.mjml`, `<name>.subject` and `<name>.sample.json`, sharing `layout.mjml` and `macros.mjml`. minijinja (strict undefined, HTML autoescape) fills them, `mrml` turns the MJML into HTML, and `html2text` makes the plain-text part. Every template gets `site.app_url` and `site.instance`
- `lettre` sends over SMTP with a 20 s timeout per step, a `Message-ID` on every message, and HTML plus plain-text alternatives
- `email_outbox` (`migrations/040_email_outbox.sql`) stores rendered emails, so a later template change never alters a queued one. Handlers call `mail::enqueue` (`crates/api/src/mail.rs`), which returns `Disabled` when mail is off and never sends inline
- The API runs a sender loop next to maintenance: every 5 s it claims up to 20 due emails under a five-minute lease (`FOR UPDATE SKIP LOCKED`), renews the lease before each send, and gives each send 60 s. A failure retries after 30 s, doubling to at most 1 h; an email still unsent 24 h after it was queued is marked failed. A crashed sender's claims come back when the lease expires
- Delivery is at least once: if the process stops after the SMTP server accepted an email but before it is marked sent, it is sent again once the lease expires
- Notification emails (`migrations/044_notification_emails.sql`, `crates/db/src/repo/notification_email.rs`, `crates/api/src/notification_mail.rs`): `user_notification_email_prefs (user_id, kind, enabled)` holds each user's choice per kind; a missing row means the kind's default (`NotificationKind::email_by_default`: share invites and moderation changes on, the rest off). `insert_notification` sets the new row's `email_state` to `pending` when the recipient's address is verified and the kind is on for them, else `none`. Each sender tick, before delivering, claims up to 50 pending rows (`FOR UPDATE SKIP LOCKED`), marks them `queued`, drops any whose recipient has since lost verification or blocked or muted the actor, renders `share_invite` or `notification` and enqueues them. A claimed row is queued at most once: an error loses that email rather than repeating it. With mail off each tick sets every pending row to `none`, so turning mail on later sends no backlog. `GET /api/users/me/notification-emails` lists every kind with its label, value and default; `PUT /api/users/me/notification-emails/{kind} {enabled}` sets one (400 for an unknown kind)
- Invite emails: the folder and dashboard share routes email a new pending invite themselves, because only the creating request has its claim token: `share_invite_pending` with `{APP_URL}/register?share=<token>`, unless the address's account turned share invite email off. A share to a verified account relies on its `share_invite` notification email. No email for a declined invitee, a block or an access change. Each owner gets 20 invite emails per hour (`INVITE_EMAIL_OWNER`, `AppState::invite_limiter`), counted in memory and reset on restart; over the limit the share is still created and its notification email is cancelled. `POST /api/categories/{id}/shares` and `POST /api/dashboards/{id}/shares` add `email_sent` to the response, and `email_limited: true` when the limit stopped the email
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
