# Invite and Notification Emails Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Email share invites (to accounts and to pending addresses) and, per the user's preferences, other notifications.

**Architecture:** Notifications are inserted in `crates/db` from many call sites, but emails are rendered in the API (`crate::mail::enqueue`). So the database marks each new notification `email_state = 'pending'` when its recipient wants email for that kind (and `'none'` otherwise); an API worker, alongside the outbox sender, renders pending ones into the outbox and marks them `'queued'`. Pending-address invites carry a raw claim token that only the creating request has, so the share routes email those directly.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8), `linkdean-mail`, React + Mantine 9 + TanStack Query + zod, vitest.

Spec: [`docs/superpowers/specs/2026-11-03-invite-and-notification-emails-design.md`](../specs/2026-11-03-invite-and-notification-emails-design.md). Outbox worker pattern: `crates/api/src/mail.rs` and the `lease-bounded-outbox-worker` skill.

**Repo rules:** as in [`2026-11-03-email-verification.md`](./2026-11-03-email-verification.md) ("Repo rules for every task"). Verify every signature against the code; report deviations.

---

### Task 1: Database and templates

- [ ] **Migration** (next free number) `…_notification_emails.sql`:

```sql
-- Per-kind email preference; a missing row means the kind's default.
CREATE TABLE user_notification_email_prefs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (user_id, kind)
);

-- Whether a notification should also be emailed: none, pending (waiting for the mailer), queued.
ALTER TABLE notifications ADD COLUMN email_state TEXT NOT NULL DEFAULT 'none'
    CHECK (email_state IN ('none', 'pending', 'queued'));
CREATE INDEX notifications_email_pending ON notifications (created_at) WHERE email_state = 'pending';
```

- [ ] **Defaults** in `crates/core` next to `NotificationKind`: `pub fn email_by_default(self) -> bool` — `ShareInvite` and `ModerationChanges` true, all others false. A user-facing label per kind for settings and the email footer (`pub fn label(self) -> &'static str`, e.g. "Share invites", "Share access changes", "Revoked shares", "Edits in shared folders", "Likes", "Comments", "Ratings", "New followers", "Moderation changes").
- [ ] **Repo** `crates/db/src/repo/notification_email.rs`:
  - `get_notification_email_prefs(pool, user_id) -> Result<Vec<(NotificationKind, bool)>>` (every kind, defaults filled in);
  - `set_notification_email_pref(pool, user_id, kind, enabled) -> Result<()>` (upsert);
  - `claim_pending_notification_emails(pool, limit) -> Result<Vec<PendingNotificationEmail>>`: `UPDATE notifications SET email_state = 'queued' WHERE id IN (SELECT id … WHERE email_state = 'pending' ORDER BY created_at LIMIT $1 FOR UPDATE SKIP LOCKED) RETURNING …` joined with the recipient's email and verification; fields: id, recipient email, recipient verified, kind, message text, subject type/id. (At-most-once per notification is fine: a lost email is acceptable, a duplicate is not worse than today.)
  - In `insert_notification` (`notifications.rs`), set `email_state` to `'pending'` when the recipient's pref for the kind (or its default) is on and the recipient's address is verified; else `'none'`. One extra query per notification; keep it inside the existing insert as a subquery if simple.
- [ ] **Templates** (`crates/mail/templates/`, pattern of `password_changed`): `share_invite` {owner, subject_kind ("folder"|"dashboard"), name, access, url}; `share_invite_pending` {owner, subject_kind, name, access, claim_url}; `notification` {heading, message, url, kind_label, settings_url}. Footer of `notification`: "You got this because email for {{ kind_label }} is on. Turn it off in Settings → Notifications." linking `settings_url`. Register, samples, tests (names escaped: a name with `<b>` renders as text).
- [ ] **Tests** `crates/db/tests/notification_email.rs`: defaults; override; a share invite notification to a verified user is `pending`, to a user with the kind off is `none`; claim marks `queued` and returns each once; muted/blocked rules unchanged (no notification → no email).
- [ ] Full suite, fmt. **Commit** "Store notification email preferences."

### Task 2: API

- [ ] **Mailer worker** in `crates/api/src/mail.rs` (or a sibling module): every outbox tick (reuse its loop and interval), when mail is on, `claim_pending_notification_emails(50)`, render (`share_invite` for `ShareInvite`, else `notification`), `enqueue` to the outbox. URLs: `/shared` for share kinds, the subject's page otherwise (check the web routes; fall back to `/`). Settings URL `{APP_URL}/settings?tab=notifications` (check the settings route and tab param). With mail off, leave rows `pending`? No: with mail off, mark nothing pending in the first place — have `insert_notification` read a flag? The DB layer cannot see mail config, so instead the worker, when mail is off, marks pending rows `none` in batches (so turning mail on later does not send a backlog). Document this.
- [ ] **Pending invite emails:** in the folder and dashboard share routes (find where `CreateCategoryShareOutcome::Pending` / the dashboard equivalent returns `claim_token` to the owner), when mail is on and the outcome is `Pending` with a fresh token, enqueue `share_invite_pending` with `claim_url = {APP_URL}/register?share=<token>` (check how the web builds the claim link today and use the same URL).
  - Skip when the invite is a repeat to an address that declined (check how `upsert_pending_share_invite` reports "already existed"/declined), and when only the access changed.
- [ ] **Rate limit:** 20 invite emails per owner per hour. Check whether `state.limiter` supports a custom window; if it only has the 15-minute window, use 5 per 15 minutes and say so in the docs. Over the limit: the share is still created; add `"email_sent": false, "email_limited": true` to the response. Under: `"email_sent": true`. Mail off: `"email_sent": false`.
- [ ] **Prefs routes:** `GET /users/me/notification-emails` → `{prefs: [{kind, label, enabled, default}]}`; `PUT /users/me/notification-emails/{kind} {enabled}`.
- [ ] **Smoke** (throwaway stack as in earlier parts: `/tmp/claude-502/ldreset/run.sh`, `smoke` DB, Mailpit `ld-mailpit`, users `uiqa-notify-*`): share to a verified account → `share_invite` mail within one outbox tick; share to a new address → `share_invite_pending` with a claim link that registers and claims; decline then re-share → no email; 21st invite in an hour → limited; turn on "Comments" → comment → `notification` mail; with it off → none; mail off → no emails and no pending backlog after turning mail on. Clean up.
- [ ] Full suite, fmt. **Commit** "Email share invites and notifications."

### Task 3: Web, docs, browser check

- [ ] **Web:** Settings → Notifications section (find the settings tabs): one Switch per kind with its label, "(on by default)" hint, saving on toggle; hidden with a note "Email notifications need outbound email" when `mail_enabled` is false. Share dialogs: after creating a share, if `email_limited`, show the spec's warning; if `email_sent`, say "We emailed them an invite." Tests for both.
- [ ] **Docs** (skill `linkdean-self-host-docs`): FEATURES (Sharing, Notifications), ARCHITECTURE (notification email states and worker, invite emails, limit), SELF_HOSTED `#email` (what mail sends now). ROADMAP item 1: remove the last bullet; if nothing of item 1 remains, remove the item and renumber, or mark it done per the repo's convention. Backlog: add "Notification digests and one-click unsubscribe (`List-Unsubscribe`)".
- [ ] **Browser check** (skill `browser-ui-verification`): share a folder with a new address → Mailpit invite → open the claim link → register → invite visible → accept; settings toggles persist; no unexplained 4xx or console errors; screenshots looked at. Clean up.
- [ ] Web checks green. **Commits** "Add notification email settings to the web app." and "Document invite and notification emails." Run `graphify update .`.
