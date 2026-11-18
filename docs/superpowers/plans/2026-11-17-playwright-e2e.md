# Playwright e2e Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Playwright suite for Linkdean web with a full skipped inventory, then implement `@smoke` on the throwaway stack and wire it to PR CI.

**Architecture:** Specs live in `apps/web/e2e/` (`*.spec.ts` only). Fixtures seed users via API + smoke DB SQL. Auth uses Playwright `storageState`. PR CI runs Chromium `@smoke` against throwaway Postgres/Meili/API/Vite. Staging is optional later via `PLAYWRIGHT_BASE_URL`.

**Tech Stack:** `@playwright/test` 1.65, Vite 5173/5199, Linkdean API, throwaway `smoke` DB, Meilisearch.

**Spec:** `docs/superpowers/specs/2026-11-17-playwright-e2e-design.md`

---

## File map

| Path | Role |
| --- | --- |
| `apps/web/playwright.config.ts` | Projects, baseURL, webServer, smoke grep |
| `apps/web/e2e/*.spec.ts` | Inventory + implemented journeys |
| `apps/web/e2e/fixtures/*` | API register, SQL ready, storageState |
| `apps/web/e2e/auth.setup.ts` | Signed-in state for smoke |
| `.github/workflows/ci.yml` | Optional e2e job (phase 3) |
| `docs/CHANGELOG.md` | Note e2e smoke when CI lands |

---

### Task 1: Inventory stubs

- [x] `apps/web/e2e/**/*.spec.ts` — **102** `test.skip` cases (`pnpm exec playwright test --list`)
- [x] `@smoke` set (10): AUTH-01, AUTH-02, ONB-01, LIB-01, LIB-02, LIB-03, SRCH-01, DASH-01, TRASH-01, VAULT-01
- [x] Commit: `Add Playwright e2e inventory as skipped journey stubs.` (`f74b726`)

Full journey catalog also recorded by explore pass (AUTH…ADM); stubs are the source of truth going forward.

### Task 2: Fixtures + auth setup

Grounding (from auth exploration): `POST /api/auth/register` + `X-Linkdean-Client: web`; `terms_version` from `GET /api/auth/bootstrap-status`; ready via `POST /api/auth/onboarding/complete` (and SQL `email_verified_at` only if verified-email gate is on); seed `localStorage.linkdean_signed_in='1'` before `storageState` or the SPA treats the session as signed out. Throwaway ports: Vite **5199**, API **18081**, DB **smoke** on `pg-check`. Login UI: `getByLabel('Email'|'Password')`, `Sign in`.

- [x] `e2e/fixtures/api.ts` — bootstrap-status → register (unique `e2e-*` email) or login; client header on mutating calls
- [x] `e2e/auth.setup.ts` — onboarding/complete + localStorage marker → `e2e/.auth/user.json`
- [x] Gitignore `e2e/.auth/`; config defaults to 5199 / API 18081; setup project + chromium `dependencies` (Vite `--host 127.0.0.1`)
- [ ] Optional: smoke-only project with `grep: /@smoke/` once P0 is implemented
- [x] Commit: `Add Playwright auth fixtures and smoke project wiring.` (`8a5ad96`, follow-up `ff9f4a9`)
- [x] Verified green: `pnpm exec playwright test --project=setup` against throwaway API :18081 / smoke DB

### Task 3: Implement P0 @smoke (un-skip + fill)

Implement in order (each green before next):

- [x] AUTH-01 / AUTH-02
- [x] ONB-01 (walks a fresh non-first user through Vault skip + Import skip; the first-admin steps need an empty DB)
- [x] LIB-01 / LIB-03
- [ ] LIB-02 — `test.fixme`: no on-blur URL validation in `BookmarkDrawer`, and the API saves `https://not a url`. Flip to `test` once the UI validates.
- [x] SRCH-01
- [x] DASH-01
- [x] TRASH-01
- [x] VAULT-01 (asserts the real no-vault modal: only "Out of vault"; the UI has no "Set up vault" prompt yet)
- [x] Commit per 1–2 green specs: `Implement @smoke …`
- Note: `POST /api/auth/register` is limited to 10 per 15 min per IP, so each run registers only twice (setup + ONB-01); AUTH-01 reuses the setup user via `e2e/.auth/credentials.json`.

### Task 4: Throwaway runner docs/script

- [x] `apps/web/e2e/README.md` — how to bring up throwaway API/DB/Meili and run `pnpm test:e2e -- --grep @smoke`
- [x] Align `playwright.config` baseURL/port with smoke skill (5199 vs 5173) via env
- [x] Commit: `Document throwaway Playwright smoke runner.`

### Task 5: PR CI job

- [x] Add CI job: services (postgres, meili) → migrate → API → `pnpm test:e2e --grep @smoke`
- [x] Artifacts: playwright report on failure (`playwright-report` + `test-results`)
- [x] CHANGELOG under Unreleased
- [x] Commit: `Run Playwright @smoke on PR CI.`

### Task 6: P1 (after P0 green in CI)

- [ ] Un-skip and implement vault create/unlock/move, share invite accept, filing queue happy path, trash restore
- [ ] Commit per journey area

### Task 7: P2 (later)

- [ ] Archive, admin screenshots/reindex, public list, passkey/SSO as feasible

---

## Verification

- Local: throwaway up → `cd apps/web && pnpm test:e2e --grep @smoke` green
- `pnpm exec playwright test --list` still lists skipped inventory
- CI job red on intentional break of Add bookmark

## Notes for agents

- Never use the dev database.
- Prefer `getByRole` / labels; add `data-testid` only if Mantine is unstable.
- No `waitForTimeout`.
- Graphify: `graphify query` before exploration; `graphify update .` after edits.
- Linkdean commits: meaningful chunks on `chore/playwright-web`; `GH_TOKEN=$(gh auth token --user arsamsarabi)` for gh.
