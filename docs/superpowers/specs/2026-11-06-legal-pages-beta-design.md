# Legal pages and beta notice

Roadmap **#6**. One PR (`feat/legal-pages-beta`).

## Goals

- Public Terms of Service, Privacy Policy, and Acceptable Use pages.
- Sign-up records terms **version** + **accepted_at**.
- Visible beta notice (no uptime promise; export regularly).
- Privacy states only strictly necessary cookies (no consent banner).

## Legal pages

SPA routes (unauthenticated): `/terms`, `/privacy`, `/acceptable-use`.

- Presentational pages with Linkdean beta copy (operator: Linkdean; contact via instance admin / future marketing site).
- Footer links on login + register; links from beta banner.
- Known issue: text is product-draft, not counsel-reviewed — replace before charging money.

## Terms acceptance

- Constant `TERMS_VERSION = "2026-11-06"` (shared: core crate + web).
- Migration `048`: `users.terms_version TEXT`, `users.terms_accepted_at TIMESTAMPTZ`.
- Password register: required checkbox; body must send `terms_version` matching server constant; store version + `now()`.
- OIDC new users: login page checkbox enables SSO start; `GET /auth/oidc/start?terms_version=…` required and must match; recorded on `create_user_gated`. Existing users signing in: no re-prompt.
- Refuse register / OIDC create when version missing or mismatched.

## Beta notice

- Dismissible banner (sessionStorage) on `__root` so login/register/app/public list all see it.
- Copy: public beta; no uptime promise; export your data regularly; links to Terms / Privacy.

## Out of scope

- Marketing site (`apps/site`)
- Re-accept flow when TERMS_VERSION bumps (admin can force later)
- Cookie consent banner
