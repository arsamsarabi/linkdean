# Moderation Design

## Goal

Let admins ban and unban accounts, manage bans in bulk, tell banned users why, and take down what a banned user has exposed to others.

## Decisions

- A ban stops the account from using the app and the API. It does not hide the account's content by itself.
- Admins can act on a banned user's content by hand, from a dedicated tool. The tool works only while the ban is active.
- Admins see and act on only what the user already exposes to others. They never see private items or vault content.
- Admins hide, revoke or remove. They never edit a bookmark's title, URL or notes.
- "Remove" moves bookmarks and categories to the owner's Trash. Comments have no Trash, so removing one deletes it and keeps the text in the audit row.
- Every ban change and content action is recorded.

## Schema

Migration `032_moderation.sql`.

`user_bans`:

| Column | Notes |
| --- | --- |
| `id` | Primary key |
| `user_id` | Banned account, cascades on user delete |
| `reason` | Required, 1 to 500 characters, shown to the user |
| `ends_at` | Null means permanent |
| `created_by` | Admin, set null on delete |
| `created_at`, `updated_at` | |
| `lifted_at`, `lifted_by` | Set by an unban |

A ban is active when `lifted_at IS NULL AND (ends_at IS NULL OR ends_at > now())`. Expiry is computed at check time, so no job runs. An index on `user_id` where `lifted_at IS NULL` serves the check.

`moderation_actions`:

| Column | Notes |
| --- | --- |
| `id` | Primary key |
| `user_id` | Moderated account, cascades on user delete |
| `admin_id` | Set null on delete |
| `action` | `ban`, `ban_changed`, `unban`, `public_link_disabled`, `category_private`, `profile_hidden`, `share_revoked`, `comment_deleted`, `bookmark_private`, `removed_to_trash` |
| `subject_type`, `subject_id` | Null for ban rows |
| `subject_title` | Snapshot, so the row stays readable after the item goes. Holds the body for a deleted comment. |
| `reason` | Admin's note |
| `created_at` | |

The migration also adds a `moderation_changes` kind to `notifications_kind_check`.

## Enforcing a ban

Sessions and `ldk_` API keys both resolve in `db::user_from_token`, and every protected route goes through the `AuthUser` extractor. `AdminUser` wraps it.

- `AuthUser` checks for an active ban after resolving the user. An active ban returns 403 with `{ "error": "banned", "code": "banned", "ban": { "reason", "ends_at" } }`, through a new `Error::Banned`.
- Login, OIDC sign-in and registration need no check of their own. They issue a session, the first authenticated request gets the 403, and the web app shows the ban.
- Sessions and API keys are kept, so an expired ban needs no new sign-in or new keys. They cannot be used while the ban is active.
- `/auth/logout` resolves the token directly, so a banned user can still sign out.
- Banning clears the user's vault keys from memory with `clear_vault_keys`. They unlock again after the ban.
- A banned email cannot register again, because the existing email conflict applies. A new email is not blocked.

### Rules

- Admins cannot ban themselves.
- A ban is refused when it would leave no admin without an active ban. The last-admin count in `update_admin_user` uses the same rule.
- Banning someone with an active ban returns 409. A new ban after one has expired or been lifted creates a new row.
- `ends_at` must be in the future.
- An admin gives a date, a number of days from 1 to 3650, or neither for permanent. The server stores a date.

## Admin API

All routes use `AdminUser`.

| Route | Purpose |
| --- | --- |
| `POST /admin/users/{id}/ban` | Ban with reason and duration |
| `GET /admin/bans` | Active bans with user, reason, end date, admin |
| `PATCH /admin/bans` | Bulk change reason and duration for `user_ids` |
| `POST /admin/bans/lift` | Bulk unban `user_ids` |
| `GET /admin/users/{id}/exposed-content` | Items others can see, grouped |
| `POST /admin/users/{id}/moderate` | Apply a list of content actions |
| `GET /admin/users/{id}/moderation-actions` | Audit list for that user |

Bulk routes run in one transaction: all succeed or none do. `/exposed-content` and `/moderate` return 409 unless the user has an active ban, so they cannot be used to browse a normal account. The audit list stays readable after a ban ends.

`GET /admin/users` gains `banned_until` and `banned`, so the Accounts list can show state.

## Content tool

Exposed content, in groups:

| Group | Source | Actions |
| --- | --- | --- |
| Public links | `categories.public_slug` | Turn off |
| Visible categories | `categories.visibility` not `private` | Set private, remove to Trash |
| Bookmarks in exposed categories | Not `is_private`, in a category above or a shared one | Set private, remove to Trash |
| Profile | `users.profile_visibility` not `hidden` | Set hidden |
| Shares | `category_shares`, `dashboard_shares`, `pending_share_invites` | Revoke |
| Comments | `social_comments` by the user | Delete |

The query excludes vault rows, deleted rows and private bookmarks. Bookmarks in the list return title and URL, the same fields another member would see.

`/moderate` takes `{ reason, actions: [{ action, subject_id }] }` in one transaction, and rechecks that each subject belongs to the user. An item already changed is skipped, not an error, so two admins cannot fail each other.

It writes one audit row per item and one `moderation_changes` notification for the user, with the counts per action and the reason. The user sees it when they return, whether the ban was lifted or expired.

Changes stay after the ban ends. The owner can restore items from Trash, or publish again. An admin can ban again if the content comes back.

`purge_expired_trash` skips users with an active ban, so a removal can be undone for as long as the ban lasts. Retention counts from the original deletion, so items already past their window go on the next purge after the ban ends.

## Web

### Admin

- Settings → Admin → Accounts rows gain a Ban button, or Unban with the end date when banned. The Ban modal asks for a reason and for Permanent, N days or a date.
- A new Banned users card sits in the right column above Accounts, hidden when nobody is banned. Rows are selectable. The bulk bar offers Unban, Change duration and Change reason.
- Each row opens a drawer with the Exposed content groups, bulk selection per group, an action bar with a required note, and the audit list. Removals and comment deletion use a confirm dialog.

### Banned user

- New `/banned` route outside the app shell, showing the reason, the end date or "permanent", and Sign out.
- `req` in `api/client.ts` sends any 403 with `code: "banned"` to `/banned`, so it works from any page and mid-session.
- The page calls `/auth/me` and reads the ban from the 403 body. A 401 goes to `/login`, and a success goes to the landing page, which covers a ban that ended.

## Edge cases

| Case | Behaviour |
| --- | --- |
| Ban expires while the tab is open | The next request works. `/banned` returns to the app when opened or refreshed. |
| Admin is banned by another admin | Same as anyone, including the admin routes. Ban expiry or another admin restores access. |
| Banned user has an unlocked vault | Keys are cleared at ban time |
| Banned user's public link | Stays live until an admin turns it off |
| Others share to a banned user | Allowed, it waits for them |
| Worker jobs for a banned user | Keep running: link checks and metadata do not depend on the owner |
| Moderate on an item the owner removed | Skipped |
| API key request while banned | The same 403 JSON |

## Testing

- DB: active ban logic with lifted, expired and permanent rows; last-admin and self-ban refusals; ban refused while one is active; bulk change rolls back on a bad id; exposed content leaves out private, vault and deleted rows; `/moderate` refuses a subject owned by someone else; each action writes an audit row and one notification; retention skips an active ban.
- API: session and `ldk_` key both get the banned 403 JSON; logout still works; `/exposed-content` and `/moderate` return 409 without an active ban and 403 for non-admins.
- Web: `req` redirects on the banned 403 only; `/banned` shows reason and end date, and leaves on 401 or success; Ban modal validates reason and duration; Banned users card hides when empty and sends bulk ids.
- Browser: ban a test account, see `/banned` mid-session, take down a public link, unban, see the notification.

## Out of scope

- IP or email-domain bans
- Appeals
- Email notices, until SMTP exists
- Admin access to private or vault content
- Hiding content automatically on ban
- A site-wide moderation log page
