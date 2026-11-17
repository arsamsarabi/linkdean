# Playwright e2e design (Linkdean web)

**Status:** approved direction (2026-11-17) — hybrid throwaway for PR; inventory as `test.skip` stubs; implement by phase.

## Goals

- Catch broken **user journeys** early, preferably on every PR.
- Keep Vitest for components/hooks; Playwright for thin, high-signal browser flows.
- Single backlog source of truth: `apps/web/e2e/**/*.spec.ts` with `test.skip` until implemented.

## Environment

| Mode | Stack | When |
| --- | --- | --- |
| PR CI / local gate | Throwaway (smoke DB, Meili, API, Vite). Never the dev DB. | Default |
| Authoring / nightly | Optional `PLAYWRIGHT_BASE_URL` → staging with dedicated e2e tenant | Later |

Throwaway rules follow `.claude/skills/linkdean-throwaway-smoke-stack/SKILL.md`: Vite **5199** → API **18081**, `X-Linkdean-Client: web`, register then `POST /auth/onboarding/complete` (SQL `email_verified_at` only when the verified-email gate is on). Playwright `storageState` must also set `localStorage.linkdean_signed_in='1'`. Never the dev DB (`5433`).

## Inventory format

- `test.describe` per area; `test.skip('ID @tag @phase: title — assert …', async () => {})`.
- Phases: `@smoke` (P0 PR), `@p1`, `@p2`.
- No parallel markdown checklist.

## Phases

1. **P0 @smoke** — ~8–12 journeys; Chromium; PR CI job; auth storageState + API seed fixtures.
2. **P1** — vault unlock/move, share accept, filing queue, dashboard edit smoke, trash restore.
3. **P2** — admin, archive, passkeys/SSO edges, deep share ACL, broken-links worker UX.

## Non-goals (v1)

- Extension e2e, visual regression matrix, Firefox/WebKit on every PR.
