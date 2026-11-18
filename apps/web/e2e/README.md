# Playwright e2e (web)

Specs run against the **throwaway smoke stack**, never the `linkdean-*` dev database.
Full rules: `.claude/skills/linkdean-throwaway-smoke-stack/SKILL.md`.

| Piece       | Default                                                                 |
| ----------- | ----------------------------------------------------------------------- |
| Vite (web)  | `127.0.0.1:5199` (`PLAYWRIGHT_WEB_PORT`, `PLAYWRIGHT_BASE_URL`)         |
| API         | `BIND=127.0.0.1:18081` (`API_PROXY_TARGET`)                             |
| Database    | `smoke` on `pg-check` (`localhost:55432`)                               |
| Meilisearch | `ld-meili-smoke` on `127.0.0.1:57700` (needed for `@smoke` search)      |
| Mailpit     | `ld-mailpit` on `11025` / UI `18025` (optional but matches smoke skill) |

## Bring up the throwaway API

Never point this at the dev DB (`5433` / `linkdean-*`).

```sh
# Postgres: recreate smoke DB on pg-check
docker exec pg-check psql -U linkdean -d postgres -c 'DROP DATABASE IF EXISTS smoke WITH (FORCE)'
docker exec pg-check psql -U linkdean -d postgres -c 'CREATE DATABASE smoke OWNER linkdean'

# Meili (search specs)
docker run -d --rm --name ld-meili-smoke \
  -p 127.0.0.1:57700:7700 \
  -e MEILI_MASTER_KEY=smoke-master-key-0001 \
  -e MEILI_NO_ANALYTICS=true \
  getmeili/meilisearch:v1.11

# Mailpit (optional)
docker run -d --rm --name ld-mailpit \
  -p 127.0.0.1:11025:1025 -p 127.0.0.1:18025:8025 \
  axllent/mailpit:v1.31.0

# API (build once: cargo build -p linkdean-api)
export \
  BIND=127.0.0.1:18081 \
  DATABASE_URL='postgres://linkdean:linkdean@127.0.0.1:55432/smoke' \
  SESSION_SECRET='e2e-session-secret-not-default-16+' \
  MEILI_URL='http://127.0.0.1:57700' \
  MEILI_MASTER_KEY='smoke-master-key-0001' \
  APP_URL='http://127.0.0.1:5199/' \
  ALLOW_DEFAULT_SECRETS=true \
  BROWSER_WS_URL='http://127.0.0.1:59998' \
  TERMS_VERSION='2026-11-15' \
  TERMS_URL='https://example.com/terms' \
  PRIVACY_URL='https://example.com/privacy' \
  SMTP_HOST=127.0.0.1 SMTP_PORT=11025 \
  MAIL_FROM='Linkdean Smoke <smoke@example.test>'
./target/debug/linkdean-api
```

Confirm `GET http://127.0.0.1:18081/api/health` returns 200 before running tests.

## Run

From `apps/web`:

```sh
pnpm exec playwright test --list              # collection only; no stack
pnpm exec playwright test --grep @smoke       # P0 smoke (setup + chromium)
pnpm test:e2e -- --grep @smoke                # same via package script
```

Playwright starts Vite itself (`webServer` on `:5199`, `--host 127.0.0.1`) and reuses one
already running outside CI. The API is **not** started for you; `auth.setup.ts` fails with a
"start the throwaway API" message if it cannot be reached.

## Auth

The `setup` project (`auth.setup.ts`) registers `e2e-<uuid>@example.test` through
`/api/auth/register` (sending `X-Linkdean-Client: web` and the server's `terms_version`),
completes onboarding, sets `localStorage.linkdean_signed_in = '1'`, and writes
`e2e/.auth/user.json` plus `e2e/.auth/credentials.json` (gitignored). The `chromium` project
depends on it and loads that storage state. Helpers live in `fixtures/api.ts`.

If the smoke DB is empty and the API sets `BOOTSTRAP_TOKEN`, export `E2E_BOOTSTRAP_TOKEN`
so the first (admin) account can register. Later accounts need registration open.

## Registration rate limit

`POST /api/auth/register` allows **10 per 15 minutes per IP**, counted in API memory. Each full
`@smoke` run registers twice (setup + `ONB-01`). After several reruns you get `429 too many
attempts`: wait out the window, or restart the throwaway API (not the dev stack) to reset the
counter. Use `--project=chromium --no-deps` to rerun specs without re-running setup.

## Archive specs (`@archive`)

`archive.spec.ts` seeds a finished HTML copy with SQL + a file under `ARCHIVE_DIR`
(default `data/archives` at the repo root) so tests do not need a live Chrome capture.
The seed helper uses `E2E_DATABASE_URL` or `DATABASE_URL` with `psql` when set; otherwise
`docker exec pg-check psql … -d smoke`. Point `E2E_ARCHIVE_DIR` at the API's `ARCHIVE_DIR`
if it is not the repo-root default.

```sh
pnpm exec playwright test e2e/archive.spec.ts --project=chromium
```

## Cleanup

Stop the throwaway API, `docker stop ld-meili-smoke ld-mailpit`, and optionally
`DROP DATABASE IF EXISTS smoke WITH (FORCE)` on `pg-check`. Do not stop `linkdean-*` or Chrome on `:9222`.
