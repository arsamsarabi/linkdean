# Single Sign-On Account Linking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An OIDC sign-in whose address belongs to a password account links to it after one password sign-in instead of being refused; signed-in users can connect and disconnect single sign-on in settings.

**Architecture:** A `pending_oidc_links` table holds the provider identity for 10 minutes, keyed by the hash of a token kept in an HttpOnly cookie. The OIDC callback creates one instead of returning 409; the password login consumes it when the cookie is present and the signed-in account holds the provider's address. A "connect" flow mode attaches the identity to the signed-in account directly. Disconnect clears the identity columns after a password check.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8, `openidconnect` as already used in `crates/api/src/routes/auth/oidc.rs`), `linkdean-mail`, React + Mantine 9 + TanStack Router/Query + zod, vitest.

Spec: [`docs/superpowers/specs/2026-11-03-oidc-account-linking-design.md`](../specs/2026-11-03-oidc-account-linking-design.md). Load the `linkdean-auth-recovery-tokens` skill when touching tokens.

**Repo rules for every task:** as in [`2026-11-03-email-verification.md`](./2026-11-03-email-verification.md) ("Repo rules for every task"): main, commit per green task and push, normal-prose commits ending `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`, stage only your files, `cargo … -j 2`, tests on `linkdean_test` at `localhost:55432` only, fmt in `rust:1.89`, web checks from `apps/web`, `npx tsr generate` after adding routes, American spelling, `graphify update .` after changes, never the dev DB or containers named `linkdean-*`. Everything names real code as of 619daab; verify signatures and report deviations.

---

### Task 1: Database

- [ ] **Migration** `migrations/043_pending_oidc_links.sql` (next free number):

```sql
-- An SSO sign-in waiting for a password sign-in to attach it to an existing account.
CREATE TABLE pending_oidc_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash TEXT NOT NULL UNIQUE,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT NOT NULL,
    email_verified BOOLEAN NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Check that `users` has a unique constraint or index on `(oidc_issuer, oidc_subject)`; if not, add a partial unique index `WHERE oidc_subject IS NOT NULL` in this migration (check existing data can't violate it: it can't, since sign-in looks up by subject).

- [ ] **Repo** `crates/db/src/repo/oidc_links.rs` (register in `repo/mod.rs`):
  - `create_pending_oidc_link(pool, issuer, subject, email, email_verified) -> Result<String>`: 32 random bytes hex token, store `hash_token`, 10-minute expiry (`pub const PENDING_OIDC_LINK_MINUTES: i64 = 10`).
  - `peek_pending_oidc_link(pool, token) -> Result<Option<PendingOidcLink>>` (live only), for the login page to show the masked address.
  - `link_oidc_identity(pool, user_id, issuer, subject) -> Result<()>`: refuses `Conflict("This sign-in is already connected to another account.")` when another user has that issuer+subject (map the unique violation too); refuses `Conflict` when this user already has a different identity; sets the columns.
  - `consume_pending_oidc_link(pool, token, user_id) -> Result<Option<PendingOidcLink>>`: in one transaction, lock the live row `FOR UPDATE`, mark used, check the user's email equals the link's email (case-insensitive) — if not, still mark used and return `Ok(None)` (nothing linked) — then `link_oidc_identity` inside the same transaction; if `email_verified`, set `email_verified_at` and attach pending invites after commit (`mark_email_verified`).
  - `unlink_oidc_identity(pool, user_id, password) -> Result<()>`: refuses accounts without a password (`BadRequest`), checks the password with the same helper `change_password` uses (`Unauthorized` on mismatch), clears both columns.
  - `purge_old_pending_oidc_links(pool) -> Result<u64>` (used or expired over a day ago); call it from `crates/api/src/maintenance.rs`.
- [ ] **Tests** `crates/db/tests/oidc_links.rs`: link once; expired refused; reuse refused; a different account consumes nothing (and the link is spent); identity already elsewhere → Conflict; account already linked to another identity → Conflict; verified provider + matching email marks verified; unlink needs the right password and a local password; purge.
- [ ] Full suite green, fmt. **Commit** "Store pending single sign-on links."

### Task 2: API and email

- [ ] **Template** `crates/mail/templates/sso_connected.{mjml,subject,sample.json}` {`provider`} (provider = issuer host): subject `Single sign-on was connected to your {{ site.instance }} account`; body "Single sign-on ({{ provider }}) was connected to your account. You can now sign in with it or with your password. Not you? Reset your password from the sign-in page and tell your admin." Button "Sign in" → `site.app_url ~ "/login"`. Register it; keep mail tests green.
- [ ] **Callback** (`oidc.rs`, around the `find_user_by_email … Conflict` known issue):
  - Flow mode: extend the flow cookie with a mode (`signin` | `connect`). `GET /auth/oidc/start?mode=connect` requires `AuthUser` and records the user id in the flow (the flow cookie is HttpOnly and short-lived; still check at the callback that the session belongs to the same user).
  - Sign-in mode, no account for the identity, a password account holds the address → `create_pending_oidc_link`, set cookie `linkdean_oidc_link` (HttpOnly, SameSite=Lax, `Secure` per `cookie_secure()`, Path=/api/auth, max-age 600), redirect to `{APP_URL}/login?link=1`. An SSO-only account (no password) with the same address but a different identity → keep a clear refusal via the existing `oidc_error` redirect.
  - Connect mode → `link_oidc_identity(current user)`; enqueue `sso_connected`; redirect `{APP_URL}/settings?sso=connected` (check the real settings route path) or with `oidc_error` on Conflict.
  - Remove the known-issue comment.
- [ ] **Login** (`routes/auth/mod.rs::login`): after a successful password sign-in, if the `linkdean_oidc_link` cookie is present, `consume_pending_oidc_link`; on `Some`, enqueue `sso_connected` (best effort) and add `"sso_connected": true` to the JSON body; always expire the cookie in the response. A failure to link never fails the login.
- [ ] **Routes:** `GET /auth/oidc/pending-link` → `{email: masked}` or 404 (reads the cookie; for the login prompt). `POST /auth/oidc/disconnect {password}` (`AuthUser`) → `unlink_oidc_identity`. `/auth/me` user JSON: add `sso_connected: bool` and `sso_provider: Option<String>` (issuer host) if not already derivable; check `UserPublic`.
- [ ] **Smoke** with a throwaway mock provider: `docker run -d --rm --name ld-oidc -p 127.0.0.1:18090:8080 ghcr.io/navikt/mock-oauth2-server:<pinned version ≥ 2 weeks old>` (check its docs for the issuer URL, e.g. `http://127.0.0.1:18090/default`, and how to choose the subject/email on its login form or via `JSON_CONFIG`). API via `/tmp/claude-502/ldreset/run.sh` with `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` added (check `crates/api/src/config/mod.rs` for the redirect URL it registers). Drive the redirects with curl and a cookie jar. Checks: password account + SSO with the same email → redirect to `/login?link=1` and cookie; pending-link endpoint shows the masked email; login with the right password → `sso_connected: true`, then an SSO sign-in alone works; wrong password → nothing linked; another account's login with the cookie → nothing linked; connect mode while signed in links; disconnect needs the password. Clean up `uiqa-sso-` users, stop the API, `docker stop ld-oidc`.
- [ ] Full suite green. **Commit** "Link single sign-on to existing accounts."

### Task 3: Web

- [ ] Login page: when `?link=1`, fetch the pending link; show an `Alert` "An account already uses {masked}. Sign in with its password to connect single sign-on." Under it, small dimmed text: "Don't know this account's password? Ask your admin." After a login whose body has `sso_connected: true`, show a notification "Single sign-on connected". If the pending link is gone (404), show nothing.
- [ ] Settings → Account (find where the change-password section lives): "Single sign-on" section, shown when `oidc_enabled`: Connected (provider host) with **Disconnect** (password modal; hidden when the account has no password), or **Connect** (link to `/api/auth/oidc/start?mode=connect`). Handle `?sso=connected` and `oidc_error` on return.
- [ ] Schemas/API/hooks for pending-link, disconnect, and the new `me` fields. Tests: login prompt shown with `link=1` and hidden on 404; settings section states; disconnect modal sends the password.
- [ ] Web checks green. **Commit** "Add single sign-on linking to the web app."

### Task 4: Docs, browser check, close-out

- [ ] Docs (skill `linkdean-self-host-docs`, values from code): SELF_HOSTED OIDC section (linking flow, connect/disconnect, the admin route for an unverified holder), ARCHITECTURE auth bullets (pending links, cookie, rules), FEATURES (Auth). ROADMAP item 1: remove the account-link bullet; only part E stays.
- [ ] Browser check (skill `browser-ui-verification`) on the throwaway stack with the mock provider: (1) password account, then SSO with the same email → login prompt → password → "Single sign-on connected" → sign out → SSO alone signs in; (2) Settings → Connect for an account with a different email; Disconnect with the password; (3) no unexplained 4xx or console errors; screenshots looked at. Clean up users, stop the API, Vite, Mailpit and `ld-oidc`.
- [ ] **Commit** "Document single sign-on account linking." Run `graphify update .`.
