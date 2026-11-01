# Moderation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Admins can ban and unban accounts in bulk, banned users see why, and admins can take down what a banned user exposes.

**Architecture:** `user_bans` holds bans and `moderation_actions` holds the audit trail. `AuthUser` is the single enforcement point. A new `moderation` repo holds the ban, exposed-content and moderate logic, each in one transaction.

**Tech Stack:** Rust, axum, sqlx, Postgres 16, React, Mantine, TanStack Query and Router, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-01-moderation-design.md`

---

Work on `main`, commit and push each green task, and end commits with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line. Run DB tests against the throwaway `linkdean-pg-check` container on port 55433. Check the migration there before saving it to `migrations/`, because the dev API applies it on save. Run `graphify update .` after each commit.

### Task 1: Bans and enforcement

**Files:** Create `migrations/032_moderation.sql`, `crates/core/src/models/moderation.rs`, `crates/db/src/repo/moderation.rs`, `crates/db/tests/moderation.rs`. Modify `crates/core/src/{error,models}/mod.rs`, `crates/api/src/{auth,error}/mod.rs`, `crates/api/src/routes/admin/mod.rs`, `crates/db/src/repo/{mod,users,trash}.rs`.

- [ ] Write DB tests: active, lifted, expired and permanent bans; self-ban and last-admin refusals; refuse while active; new ban after expiry; bulk change rolls back on a bad id; duration validation; retention skips an active ban; demoting uses the active admin count.
- [ ] Add the migration, models and `Error::Banned`.
- [ ] Add `active_ban`, `ban_user`, `change_bans`, `lift_bans`, `list_bans`, `list_moderation_actions`.
- [ ] Check the ban in `AuthUser`, and return the banned JSON from `ApiError`.
- [ ] Add `banned` and `banned_until` to `AdminUserSummary`.
- [ ] Add the admin ban routes, clearing vault keys on ban.
- [ ] Smoke test routes with a session and an `ldk_` key. Run the Rust gate and commit: `Add account bans.`

### Task 2: Exposed content and moderate

**Files:** Modify `crates/core/src/models/moderation.rs`, `crates/db/src/repo/moderation.rs`, `crates/db/tests/moderation.rs`, `crates/api/src/routes/admin/mod.rs`.

- [ ] Write DB tests: exposed content leaves out private, vault and deleted rows and includes subcategory bookmarks; each action applies, writes an audit row and one notification; repeat actions skip; a subject owned by someone else rolls back; vault subjects are refused.
- [ ] Add `exposed_content` and `moderate_content`, and the two routes with the 409 guard. After commit, remove trashed bookmarks from Meilisearch and send existing revoke notifications.
- [ ] Run the Rust gate and commit: `Add admin moderation of exposed content.`

### Task 3: Banned page and ban management

**Files:** Create `apps/web/src/routes/banned.tsx`, `apps/web/src/pages/banned/`, `apps/web/src/schemas/moderation/`, `apps/web/src/hooks/moderation/`, `apps/web/src/pages/settings/{BanUserModal,BannedUsersPanel}.tsx`. Modify `api/client.ts`, `api/api.ts`, `pages/settings/{AdminUsersPanel,AdminPlatformSettings}.tsx`, `schemas/auth/auth.ts`.

- [ ] Test the `req` redirect and the page first, then add them.
- [ ] Add the Ban modal, Accounts buttons and the Banned users card with bulk actions, with tests.
- [ ] Run the web gate and commit: `Add the banned page and ban management.`

### Task 4: Content drawer and close out

**Files:** Create `apps/web/src/pages/settings/ModerationDrawer.tsx`. Modify `pages/settings/BannedUsersPanel.tsx`, `schemas/notifications/notifications.ts`, `components/notifications/NotificationsDrawer.tsx`, `docs/ROADMAP.md`, `docs/FEATURES.md`.

- [ ] Add the drawer with groups, selection, note, confirm and audit list, with tests.
- [ ] Add the `moderation_changes` notification display.
- [ ] Browser check: ban a test account, see `/banned` mid-session, take down a public link, unban, see the notification.
- [ ] Move Moderation from the Roadmap to Features, run all gates and commit: `Add the moderation content drawer.`
