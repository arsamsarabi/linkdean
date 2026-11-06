# Account deletion and full export

Approved from roadmap **#5** (“yes / one PR / AFK”). One PR.

## Goals

- Full export of everything the person owns, including vault items **while the vault is unlocked**.
- Self-serve **delete account** in Settings, with a short grace period, then hard purge.
- Cascade wipe of owned data (bookmarks, vaults, files, shares, sessions, API keys, search entries). Owned shared folders are **deleted** (invitees lose access); no hand-off in v1.

## Full export

Extend existing `GET /export`:

- When the vault session keyring is non-empty, include vault categories and vault bookmarks as plaintext in JSON (and Netscape HTML).
- When locked, keep today’s behaviour (vault skipped) and keep the UI note.
- Still: categories, non-vault bookmarks, dashboards (pins only for exported bookmarks).
- No binary preview blobs in the file — URLs / flags only, as today.

## Delete account

### Schedule

1. `POST /users/me/deletion` with:
   - `confirm_email` must match the account email (case-insensitive).
   - Password users: `password` required and verified.
   - OIDC-only: no password; email confirm is the gate (Known issue: true fresh-OIDC step-up deferred).
2. Refuse if the user is the **last active admin** (same rule as ban).
3. Set `users.deletion_scheduled_at = now + 7 days`.
4. Email: scheduled date + how to cancel in Settings.
5. Client signs out after success (or keep session so they can cancel — **keep session** until purge so Cancel works).

### Cancel

- `DELETE /users/me/deletion` clears `deletion_scheduled_at` while still before purge.
- Settings shows banner when scheduled: date + Cancel.

### Purge (maintenance, hourly with trash)

For each user with `deletion_scheduled_at <= now`:

1. Collect owned bookmark ids (for Meili + preview cleanup).
2. `DELETE FROM users WHERE id = $1` (FK cascades own the rest).
3. Meili `delete_bookmark` per id; remove preview/favicon/og assets like trash purge.
4. Clear any in-memory vault keys for that user.

### Auth while scheduled

- Login and API still work so the person can export again and cancel.
- Optional: surface `deletion_scheduled_at` on `/me`.

## Web

- Import/Export: drop “vault always skipped” when unlocked; wording reflects lock state.
- Settings → Account (or Danger zone on Profile/General): Delete account modal (email + password if local), Cancel when scheduled.
- After schedule: notification + show scheduled date from `/me`.

## Out of scope

- Handing shared folders to another user
- Fresh OIDC re-auth redirect for delete confirm
- Exporting binary preview files or activity logs
- Admin-initiated account purge UI (ban remains separate)
