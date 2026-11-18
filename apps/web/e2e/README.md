# Playwright e2e (web)

Specs run against the **throwaway smoke stack**, never the `linkdean-*` dev database.
See `.claude/skills/linkdean-throwaway-smoke-stack/SKILL.md` for starting it.

| Piece | Default |
|---|---|
| Vite (web) | `127.0.0.1:5199` (`PLAYWRIGHT_WEB_PORT`, `PLAYWRIGHT_BASE_URL`) |
| API | `BIND=127.0.0.1:18081` (`API_PROXY_TARGET`) |
| Database | a throwaway Postgres DB (smoke DB on `pg-check`) |

## Run

1. Start the throwaway API on `127.0.0.1:18081` against the smoke DB.
2. From `apps/web`:

   ```sh
   pnpm exec playwright test --list   # collection only, needs no stack
   pnpm exec playwright test          # starts/reuses Vite on :5199
   ```

Playwright starts Vite itself (`webServer`) and reuses one already running on :5199 outside CI.
The API is **not** started for you; `auth.setup.ts` fails with a "start the throwaway API"
message if it cannot be reached.

## Auth

The `setup` project (`auth.setup.ts`) registers `e2e-<uuid>@example.test` through
`/api/auth/register` (sending `X-Linkdean-Client: web` and the server's `terms_version`),
completes onboarding, sets `localStorage.linkdean_signed_in = '1'`, and writes
`e2e/.auth/user.json` (gitignored). The `chromium` project depends on it and loads that
storage state (cookie `linkdean_session`). Helpers live in `fixtures/api.ts`.

If the smoke DB is empty and the API sets `BOOTSTRAP_TOKEN`, export `E2E_BOOTSTRAP_TOKEN`
so the first (admin) account can register. Later accounts need registration open.

`auth.setup.ts` also writes `e2e/.auth/credentials.json` (the setup user's email) so sign-in
specs reuse that account.

## Registration rate limit

`POST /api/auth/register` allows **10 per 15 minutes per IP**, counted in API memory. Each full run
registers twice (setup + `ONB-01`). After several reruns you get `429 too many attempts`: wait out
the window, or restart the throwaway API (not the dev stack) to reset the counter. Use
`--no-deps` to rerun specs without re-running setup.
