# Email Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify account email addresses, so invites only reach people who control the address; verify email changes before they apply; give admins verification controls and an optional hard gate.

**Architecture:** `users.email_verified_at` records verification (existing accounts are backfilled as verified). Part B's `password_reset_tokens` becomes `account_tokens` with a `purpose` (`password_reset`, `verify_email`, `change_email`) and an optional `new_email`; one live token per user and purpose. Shares to an unverified account become address-only pending invites, which attach when the account verifies. The hard gate is a platform setting checked in the `AuthUser` extractor, only for unverified users.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8), `linkdean-mail` (MJML), React + Mantine 9 + TanStack Router/Query + zod, vitest.

Spec: [`docs/superpowers/specs/2026-11-03-email-verification-design.md`](../specs/2026-11-03-email-verification-design.md). Patterns to copy: part B (`crates/db/src/repo/password_resets.rs`, `crates/api/src/routes/auth/password_reset.rs`, `apps/web/src/pages/password-reset/`). Load the `linkdean-auth-recovery-tokens` skill when touching tokens.

**Repo rules for every task:**

- Work on `main`. Commit each green task and push. Never commit red. Commit messages are normal prose ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Stage only the files you changed; never `git add -A` (`brand/graphics` and `graphify-out/` stay uncommitted).
- Rust: always `cargo … -j 2`. Tests use only `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test`. Smoke runs use the `smoke` database on the same server. Never the dev database; never touch containers named `linkdean-*`.
- Rust format: `docker run --rm -v "$PWD":/w -w /w rust:1.89 sh -c "rustup component add rustfmt >/dev/null 2>&1; cargo fmt --all -- --check"`.
- Web, from `apps/web`: `npx vitest run`, `npx prettier --check src`, `npx oxlint src`, `npx tsc -b --noEmit`. `routeTree.gen.ts` is tracked: regenerate with `npx tsr generate` after adding routes.
- American spelling in copy, identifiers and docs.
- After changing files, run `graphify update .` from the repo root.
- Everything in this plan names real code as of commit 60c5b4b, but verify each signature before using it and adapt; report deviations.

---

## File map

| File | Responsibility |
| --- | --- |
| `migrations/042_email_verification.sql` (create) | `email_verified_at`, token table rename + purpose, platform setting |
| `crates/db/src/repo/password_resets.rs` → `account_tokens.rs` (rename), `repo/mod.rs` | Tokens for all purposes |
| `crates/db/src/repo/email_verification.rs` (create) | Verify, email change, admin verification controls |
| `crates/db/src/repo/users.rs`, `shares.rs`, `dashboard_shares.rs`, `pending_shares.rs`, `platform.rs` (or wherever platform settings live) | Verified flag, pending routing, setting |
| `crates/core/src/models/mod.rs` | `User.email_verified_at`, `PublicUser.email_verified`, `AdminUserSummary.email_verified`, `PlatformSettings.require_verified_email`, `UpdateMe` without `email` |
| `crates/db/tests/email_verification.rs` (create), `crates/db/tests/password_resets.rs` (keep green) | Tests |
| `crates/mail/templates/verify_email.*`, `email_change_verify.*`, `email_change_notice.*`, `render.rs` | Emails |
| `crates/api/src/routes/auth/email_verification.rs` (create), `routes/auth/mod.rs`, `routes/auth/oidc.rs`, `routes/users/mod.rs`, `routes/admin/mod.rs`, `routes/` (claim), `auth/mod.rs`, `rate_limit.rs`, `error.rs` | Routes, hard gate |
| `apps/web/src/…` | Banner, verify page, change email, gate screen, admin, claim |
| Docs | README, SELF_HOSTED, ARCHITECTURE, FEATURES, ROADMAP |

---

### Task 1: Database

- [ ] **Step 1: Migration** `migrations/042_email_verification.sql` (check the next free number):

```sql
-- Email verification. Accounts that exist now count as verified.
ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;
UPDATE users SET email_verified_at = now();

-- Password reset tokens become account tokens with a purpose.
ALTER TABLE password_reset_tokens RENAME TO account_tokens;
ALTER TABLE account_tokens ADD COLUMN purpose TEXT NOT NULL DEFAULT 'password_reset'
    CHECK (purpose IN ('password_reset', 'verify_email', 'change_email'));
ALTER TABLE account_tokens ALTER COLUMN purpose DROP DEFAULT;
-- The address an email change switches to.
ALTER TABLE account_tokens ADD COLUMN new_email TEXT;
ALTER INDEX password_reset_tokens_user RENAME TO account_tokens_user;
DROP INDEX account_tokens_user;
CREATE INDEX account_tokens_user ON account_tokens (user_id, purpose) WHERE used_at IS NULL;

ALTER TABLE platform_settings ADD COLUMN require_verified_email BOOLEAN NOT NULL DEFAULT false;
```

Check the platform settings table name and the token index name in earlier migrations; adjust. Run the migration against `linkdean_test` via the tests, and check `\d account_tokens` in the `pg-check` container.

- [ ] **Step 2: Rename and generalize tokens.** `git mv crates/db/src/repo/password_resets.rs crates/db/src/repo/account_tokens.rs`; update `repo/mod.rs`. Add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose { PasswordReset, VerifyEmail, ChangeEmail }

impl TokenPurpose {
    fn as_str(self) -> &'static str {
        match self {
            Self::PasswordReset => "password_reset",
            Self::VerifyEmail => "verify_email",
            Self::ChangeEmail => "change_email",
        }
    }
}
```

- `create_account_token(pool, user_id, purpose, ttl, created_by, new_email: Option<&str>) -> Result<String>`: same body as `create_reset_token` (user row lock, void, insert), voiding and inserting only for that purpose.
- Keep `create_reset_token(pool, user_id, ttl, created_by)` as a thin wrapper (`PasswordReset`, `None`) so part B callers and tests stay unchanged.
- `void_reset_tokens` → `void_account_tokens(tx, user_id, purpose)`; `change_password` keeps voiding `PasswordReset` only.
- `consume_reset_token`: add `AND t.purpose = 'password_reset'` to its select; voids `PasswordReset` only.
- A private helper `take_account_token(tx, token, purpose) -> Result<Option<(Uuid, Option<String>)>>`: select `user_id, new_email` with `used_at IS NULL AND expires_at > now() AND purpose = $2 FOR UPDATE OF t`, then mark every live token of that user and purpose used. Use it in `consume_reset_token` and below.
- `purge_old_reset_tokens` → `purge_old_account_tokens` (all purposes); update `crates/api/src/maintenance.rs`.

- [ ] **Step 3: Verified flag.** Add `email_verified_at: Option<DateTime<Utc>>` to `User` (it is `sqlx::FromRow`; every `SELECT *` keeps working, but check explicit column lists in `users.rs`). Add `email_verified: bool` to `PublicUser` (built in `to_public`) and `AdminUserSummary` (`admin_summary`), and `require_verified_email: bool` to `PlatformSettings` and `Option<bool>` to `UpdatePlatformSettings` (update the platform settings repo).
- `create_user` / `create_user_gated`: new accounts start unverified (`NULL`). OIDC creation sets it (Task 3).

- [ ] **Step 4: `crates/db/src/repo/email_verification.rs`:**

```rust
/// Mark the account verified and attach invites waiting for its address.
pub async fn mark_email_verified(pool: &PgPool, user_id: Uuid) -> Result<u32>
// UPDATE users SET email_verified_at = coalesce(email_verified_at, now()) WHERE id = $1 RETURNING email;
// then super::pending_shares::link_pending_share_invites(pool, &email, user_id)

/// Verify with a link. Returns the user id.
pub async fn verify_email_token(pool: &PgPool, token: &str) -> Result<Uuid>
// take_account_token(VerifyEmail) in a tx → BadRequest("this verification link has expired or was already used") when None
// set email_verified_at, commit, then link pending invites (outside the tx, like mark_email_verified)

/// Start an email change: checks the password, refuses SSO-only accounts and taken addresses.
/// Returns the token for the new address.
pub async fn start_email_change(pool: &PgPool, user_id: Uuid, password: &str, new_email: &str) -> Result<String>
// normalize (trim, lowercase, must contain '@'); same as current → BadRequest; taken → Conflict("email already registered")
// verify the password with the same KDF helper change_password uses (BadRequest/Unauthorized as it does)
// create_account_token(ChangeEmail, Duration::hours(24), None, Some(&new_email))

/// Finish an email change with its link: rechecks the address is free, switches, verifies.
/// Returns (user_id, old_email, new_email).
pub async fn confirm_email_change(pool: &PgPool, token: &str) -> Result<(Uuid, String, String)>
// take_account_token(ChangeEmail) → new_email; in the same tx: conflict check, UPDATE users SET email=$2, email_verified_at=now(); void PasswordReset tokens; commit; link pending invites for new_email

/// Admin controls.
pub async fn set_email_verified(pool: &PgPool, user_id: Uuid, verified: bool) -> Result<()>
pub async fn admin_change_email(pool: &PgPool, user_id: Uuid, new_email: &str) -> Result<()>
// normalize + conflict check; sets email and email_verified_at = now(); voids ChangeEmail and PasswordReset tokens; links pending invites
```

A unique-violation on `users.email` must map to `Conflict`, not 500 (two changes racing to the same address).

- [ ] **Step 5: Close the direct email edit.** Remove `email` from `UpdateMe` (or reject it with `BadRequest("change your email from Settings → Account")`) in `users.rs::update_me` and the core model. Find the web caller (`apps/web/src/pages/profile/ProfileAboutCard.tsx` has an Email input) — Task 4 replaces it.

- [ ] **Step 6: Route invites for unverified accounts.** In `shares.rs::create_share` and `dashboard_shares.rs` (the `find_user_by_email` at its share creation), treat an invitee with `email_verified_at IS NULL` like an unknown address: create the address-only pending invite (`invitee_user_id: None`) and return `Pending`. Keep the self-share and block checks. Check `pending_shares.rs::upsert_pending_share_invite` and its `find_open_invite` for the same assumption. `link_pending_share_invites` attaches them on verification.
- Also `link_pending_share_invites` must still revoke on blocks (it does).

- [ ] **Step 7: Signed-in claim.** Add `claim_pending_share_invite_signed_in(pool, token, user_id)`: like `claim_pending_share_invite` but takes the account's own email (look it up), so a signed-in user whose address matches the invite can claim it. Reuse the existing function body rather than duplicating.

- [ ] **Step 8: Tests** `crates/db/tests/email_verification.rs` (pattern: `crates/db/tests/password_resets.rs`):
  - a new account is unverified; `verify_email_token` verifies it once; reuse and expiry are refused; a new link voids the old one;
  - a reset token cannot verify an email, and a verification token cannot reset a password (purpose separation);
  - a share to an unverified account is `Pending`; after verifying, the invite is attached (visible to the invitee as an invite);
  - a dashboard share behaves the same;
  - email change: wrong password refused; SSO-only refused; taken address `Conflict`; the email stays the old one until `confirm_email_change`; after it, email is new and verified; if another account took the address in between, confirm returns `Conflict`;
  - admin: `set_email_verified(false)` then a new share goes pending; `admin_change_email` applies at once;
  - `update_me` no longer changes the email;
  - existing `password_resets.rs` tests still pass.
- [ ] Full `cargo test -j 2 --workspace` green, fmt. **Commit** "Track email verification and hold invites for unverified accounts."

---

### Task 2: Email templates

Like part B's Task 2 (`crates/mail/templates/password_reset.*`). Register in `TEMPLATES`; samples; one test that `verify_email` text holds the whole link on one line.

| Template | Variables | Subject | Content |
| --- | --- | --- | --- |
| `verify_email` | `verify_url`, `hours` | `Verify your email for {{ site.instance }}` | "Confirm this is your address so people can share folders and dashboards with you. The link works for {{ hours }} hours." Button "Verify your email". |
| `email_change_verify` | `verify_url`, `hours` | `Confirm your new email for {{ site.instance }}` | "Someone asked to use this address for a {{ site.instance }} account. Open the link within {{ hours }} hours to switch. If this wasn't you, ignore this email." Button "Use this address". |
| `email_change_notice` | `masked_email` | `Your {{ site.instance }} email is being changed` | "Someone asked to change this account's email to {{ masked_email }}. Until the new address is confirmed, this one stays in use. Not you? Reset your password from the sign-in page." Button "Sign in" → `site.app_url ~ "/login"`. |

- [ ] Commit "Add the email verification emails."

---

### Task 3: API

- [ ] **Limits** (`rate_limit.rs`): `verify_resend_limits(user_id, ip)` → 3 per account, 10 per IP; `verify_attempt_limit(ip)` → 10 per IP. Unit tests like part B's.
- [ ] **Routes** in `crates/api/src/routes/auth/email_verification.rs` (all under `/api/auth/` so the hard gate exempts them):
  - `POST /auth/email/resend` (`AuthUser`): already verified → `{ok: true, verified: true}`; mail off → 400 "email verification needs mail; ask your admin"; else create a `VerifyEmail` token (24 h) and enqueue `verify_email` with `verify_url = {APP_URL}/verify-email#token=…`.
  - `POST /auth/email/verify` `{token}` (no auth): `verify_email_token`; → `{ok: true}`.
  - `POST /auth/email/change` `{password, new_email}` (`AuthUser`): mail off → 400 "ask your admin"; `start_email_change`; enqueue `email_change_verify` to the new address (`{APP_URL}/verify-email#token=…&change=1`) and `email_change_notice` to the old (`masked_email`: first letter, `•••`, `@domain`); → `{ok: true}`. Rate limit with the resend limits.
  - `POST /auth/email/confirm-change` `{token}` (no auth): `confirm_email_change` → `{ok: true}`.
  - Build the URL with a shared helper next to `password_reset::reset_url` (generalize it: `fragment_link(app_url, path, token)`).
- [ ] **Register:** after a successful password registration, when mail is on, enqueue `verify_email` (best effort; log on failure; never fail the registration). The `share_token` claim path stays.
- [ ] **OIDC:** when creating an account with `email_verified == Some(true)`, call `mark_email_verified` (which also links pending invites; replace the existing `link_pending_share_invites` call there so it isn't done twice). For an existing OIDC user signing in with `email_verified == Some(true)` while unverified, mark verified too.
- [ ] **`/auth/me`, bootstrap:** `user.email_verified` comes from `PublicUser`; add `require_verified_email` to `/auth/me`'s JSON.
- [ ] **Hard gate** in `crates/api/src/auth/mod.rs` `AuthUser`: after the ban check, if `user.email_verified_at.is_none()`, load platform settings; if `require_verified_email` and the request path is not exempt, return a new `Error::EmailUnverified` → `403 {"error": "email_unverified", …}` (add it in `crates/core` errors and `crates/api/src/error.rs` alongside `Banned`). Exempt: paths starting `/auth/` (check whether `parts.uri` inside the nested `/api` router is stripped; use `OriginalUri` if needed). `AdminUser` delegates to `AuthUser`, so admins are gated too, except an admin is never gated (they could lock everyone out): skip the gate when `user.is_admin`.
- [ ] **Platform setting:** in the admin platform PATCH, refuse `require_verified_email: true` when `state.mail.is_none()` → 400 "turn on outbound email first".
- [ ] **Admin routes** (`routes/admin/mod.rs`): `POST /admin/users/{id}/email-verified {verified: bool}`, `POST /admin/users/{id}/email {email}` → `admin_change_email`.
- [ ] **Signed-in claim:** `POST /api/shares/claim {token}` (`AuthUser`) → `claim_pending_share_invite_signed_in`; 400 when the address doesn't match or the token is used. Put it next to the existing pending-invite routes.
- [ ] **Profile:** `PATCH /users/me` rejects `email` (from Task 1).
- [ ] **Smoke** (as part B Task 3; script `/tmp/claude-502/ldreset/run.sh`, API on 127.0.0.1:18081, Mailpit `ld-mailpit`, `smoke` DB, test users `uiqa-verify-*`, all POSTs with `X-Linkdean-Client: web`):
  - register → `verify_email` in Mailpit → verify → `/auth/me` `email_verified: true`; reuse → 400;
  - user A shares a folder with unverified user B → pending; B verifies → B sees the invite;
  - change email: notice to old, link to new; email unchanged until confirm; after confirm, login with the new address works;
  - hard gate on (admin PATCH) → unverified user gets 403 `email_unverified` on `/api/bookmarks`, 200 on `/api/auth/me`; admin never gated; with mail off the PATCH → 400;
  - admin: remove verification → new share pending; admin change email works;
  - signed-in claim with mail off: pending invite claim token claimed by the matching signed-in account; mismatched account → 400.
  - Clean up users and outbox rows; stop API and Mailpit.
- [ ] Full suite green. **Commit** "Verify email addresses and email changes."

---

### Task 4: Web

- [ ] **Contract:** schemas (`email_verified` on the user, `require_verified_email` on `me` and platform settings, `email_verified` on admin summaries), API functions and hooks for resend, verify, change, confirm-change, admin verified/email, claim. `ApiError` with `error: 'email_unverified'` → the gate screen (handle it where the app handles `banned`, see `routes/_app.tsx` and `pages/banned`).
- [ ] **Banner:** in the app shell, when `me.user.email_verified === false` and `mail_enabled`: a slim Mantine `Alert` "Verify your email so people can share with you." with "Resend email" (shows "Sent" after success; 429 → API message). Dismissible for the session only.
- [ ] **`/verify-email` page** (public, like `/reset-password`; add to `isPublicPath` in `api/client.ts`): read `token` (and `change=1`) from the fragment, clear it with `history.replaceState`, call verify or confirm-change automatically once, show "Email verified" / "Email changed" with a button to the library (signed in) or sign-in; on failure "This link has expired or was already used." with "Send a new link" when signed in. Guard against double-calls in StrictMode (call once per token).
- [ ] **Gate screen:** when the API answers `email_unverified`, show a full-page "Verify your email" with the address, "Resend email" and "Sign out" (follow `BannedPage`).
- [ ] **Change email:** `ProfileAboutCard.tsx`'s Email input becomes read-only text with a "Change email" button (hidden for accounts without a password; when mail is off show "Ask your admin to change it"). It opens a modal: new email, current password, "Send confirmation link". After success: "Check n…@… for a link. Your email stays the same until you open it."
- [ ] **Admin:** `AdminUsersPanel.tsx`: "Unverified" badge when `email_verified` is false; actions "Mark verified" / "Remove verification" and "Change email" (a small modal). Platform panel: Switch "Require verified email" with description "Unverified accounts can only verify or sign out. Needs outbound email." disabled when mail is off.
- [ ] **Pending label:** in the folder/dashboard share dialogs, a pending invite whose address belongs to an unverified account looks the same as any pending invite; change its text to "Waiting for x@… to verify their email or accept" only if the API exposes the distinction cheaply; otherwise leave the generic pending label (note it in the report).
- [ ] **Claim while signed in:** the claim link today opens `/register?share=…`. When a signed-in user opens it, call `POST /api/shares/claim` and show the result instead of the register form.
- [ ] **Tests:** banner shows/hides (mail on/off, verified/unverified); verify page success, failure, fragment cleared; gate screen on `email_unverified`; change email modal submits password + address; admin badge and actions; platform switch disabled with mail off.
- [ ] All web checks green. **Commit** "Add email verification and email change to the web app."

---

### Task 5: Docs, browser check, close-out

- [ ] **Docs** (skill `linkdean-self-host-docs`; values from code):
  - SELF_HOSTED `#email`: verification needs mail; with mail off accounts stay unverified, invites use claim links, admins can mark verified; the hard gate needs mail. **Upgrade note:** existing accounts count as verified; remove verification from any you don't trust (Admin → Accounts).
  - ARCHITECTURE: account tokens (purposes, one live per purpose), the verified flag and invite routing, the hard gate (exempt `/api/auth/*`, admins never gated), limits.
  - FEATURES: email verification, email change, admin controls. Remove the verification bullets from ROADMAP item 1 (parts D and E stay). Update the known-issue comment on `create_share` (remove it).
- [ ] **Browser check** (skill `browser-ui-verification`, throwaway stack as in part B Task 5):
  1. Register → banner → Mailpit link → "Email verified" → banner gone.
  2. User A shares a folder with unverified B → B verifies → B sees the invite.
  3. Change email from Profile → email unchanged → open link → "Email changed" → sign in with the new address.
  4. Admin turns on "Require verified email" → unverified user sees the gate screen → Resend works → after verifying, the app loads.
  5. Mail off: no banner; switch disabled; admin marks a user verified.
  6. No unexplained 4xx, no console errors; screenshots of the banner, verify page and gate screen, looked at.
  Clean up test users, outbox rows and admin flags; stop servers and Mailpit.
- [ ] **Commit** "Document email verification." Run `graphify update .`.
