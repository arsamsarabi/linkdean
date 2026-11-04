# Organizations and Teams: Phases

Spec: [`docs/superpowers/specs/2026-11-03-organizations-and-teams-design.md`](../specs/2026-11-03-organizations-and-teams-design.md). Roadmap item **Organizations and teams** in [`docs/ROADMAP.md`](../../ROADMAP.md).

The spec is too large for one plan. It ships in five phases. Each phase leaves `main` working, tested and documented, and each one is useful on its own. Only Phase 1 has a task-by-task plan so far: [`2026-11-03-organizations-phase-1.md`](./2026-11-03-organizations-phase-1.md). Write the plan for each later phase when the one before it has shipped, against the code as it is then.

Vocabulary is the spec's, in American spelling: Organization, Workspace ("Personal" or an organization), Team, Team space, Members, Guests, Restricted folder, Invite, Instance admin, Plans; organization roles Owner, Admin, Member, Guest; team roles Maintainer, Editor, Viewer.

## How ownership works today

Every fact below was checked against the code at commit `721ca46`.

- **One owner column.** `categories` (folders), `bookmarks`, `tags` and `dashboards` each have `owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE` (`migrations/001_init.sql`, `010_dashboards.sql`). `tags` are `UNIQUE (owner_user_id, name)`, and `dashboards_one_default_per_user` is a partial unique index on `owner_user_id`. The repos filter on `owner_user_id = $user` throughout: `bookmarks.rs` (39 mentions), `categories.rs` (33), `shares.rs` (31), `dashboards.rs` (25), `dashboard_shares.rs` (21), `social.rs` (17), `pending_shares.rs` (14), `moderation.rs` (14), `trash.rs` (7), plus `tags.rs`, `import.rs`, `notifications.rs`, `blocks.rs`, `vault_moves.rs` and `profiles.rs`.
- **Access to a folder** comes from `crates/db/src/repo/shares.rs::category_access(pool, user_id, category_id) -> Result<CategoryAccess>`. The owner wins (`CategoryAccess::Owner`). A vault folder is `NotFound` to anyone else. Otherwise the code walks up `parent_id` for the nearest `category_shares` grant (`read`/`write`), and any trashed ancestor makes it `NotFound`. `require_category_read`, `require_category_write` and `require_category_owner` wrap it. `CategoryAccess` is `Owner | Read | Write` (`crates/core/src/models/mod.rs`).
- **Access to a bookmark** comes from `bookmark_can_read` and `bookmark_can_write` in `shares.rs`. They allow the owner first. A trashed bookmark is then refused, because trash belongs to the owner alone. After that, any linked folder (`bookmarks.category_id` or `bookmark_categories`) that `category_access` allows grants access. A bookmark can sit in several folders.
- **A bookmark belongs to whoever added it**, even inside someone else's write-shared folder: `create_bookmark(pool, user_id, …)` checks `require_category_write` and inserts with `owner_user_id = user_id`.
- **Dashboards** use `dashboard_shares.rs::require_dashboard_owner` and their own share table. Pending invites to addresses live in `pending_share_invites` (`subject_type` `category`|`dashboard`), with claim tokens (migration 033). `link_pending_share_invites` attaches them once an address is verified.
- **Authz tests:** `crates/db/tests/authz.rs` (A cannot reach B's rows, read/write shares, vault folders cannot be shared, public slugs, pending invites), plus `share_invites.rs`, `shared_trash.rs`, `shared_bookmark_links.rs`, `vault_moves.rs`, `blocks.rs` and `social_nested.rs`.
- **Search:** `crates/search` puts non-vault bookmarks in the Meilisearch index `bookmarks`, with `owner_user_id`, `category_id` and `category_ids` filterable. `meili_visibility_filter(user, readable_category_ids)` builds `owner_user_id = "u" OR category_id IN […] OR category_ids IN […]`, and every hit is checked again against Postgres before it is returned.
- **Activity:** `activity_events.user_id` is per user (`migrations/020`). **Notifications:** `notifications.kind` has a CHECK constraint, last rewritten in `migrations/032`. The email state comes from `migrations/044`.
- **Deleted items (trash):** `deleted_at` on bookmarks, categories and dashboards (`migrations/013`). Retention is each user's `user_settings.trash_retention_days`, and `trash.rs::purge_expired_trash` runs hourly from `crates/api/src/maintenance.rs`.
- **Import/export:** `import.rs::apply_parsed_import*` and `export_data` are per owner.
- **Instance admin** is `users.is_admin`, checked by the `AdminUser` extractor. Bans are `user_bans` (`moderation.rs::active_ban`), checked in `AuthUser`.
- **No account deletion exists** in the API today, so the spec's "A user deletes their account" row has no code to change yet.
- **Web:** the sidebar is `apps/web/src/pages/app-shell/AppSidebar.tsx` (Dashboard, Library, Shared, Vaults, Quick views, `SidebarFolders`, Library health, then Deleted items and Config options in the footer). The account menu is in `AppShellHeader.tsx`. Phones use `MobileTabBar.tsx` plus the burger, which shows the same sidebar. Settings uses `?tab=` (`pages/settings/SettingsPage.tsx`). Data goes through `api/api.ts` → `schemas/<domain>` (zod) → `hooks/<domain>` (TanStack Query). Navigation rule (skill `linkdean-web-navigation`): one control per action per width.

## Phase overview

| Phase | Ships | Touches personal code? |
| --- | --- | --- |
| 1. Foundation | Organizations, members and roles, default team, teams and team roles, email invites, organization settings (Members, Teams, Policies, minimal Audit log), audit log recording, the Plans seam, instance setting for who creates organizations, workspace switcher with empty team spaces | No: new tables and routes only |
| 2. Team spaces | Content owned by an organization and scoped to a team: folders, bookmarks, tags, dashboards; permission checks; search scoping; moving personal → team; copying team → personal; team Deleted items; capture to the current workspace | **Yes: the ownership refactor** |
| 3. Restricted folders | Narrowing a folder to some members or teams of its space; locks in the sidebar; search and activity respect restrictions | Team code only |
| 4. Lifecycle and instance admin | Archive, restore and delete teams; organization deletion with grace, restore and purge; leaving and offboarding ("Former member"); ownership transfer; instance-admin organization list, suspend and delete; the instance-wide ban shown to organizations | Account deletion guard (when account deletion exists) |
| 5. Policies, audit and parity | Policies (public sharing, who creates teams, team → personal copy, retention); audit log filters and CSV export; team activity feed; library health, import and export by maintainers; likes, comments and ratings on team content; organization logo | Moderation covers organization public links |

The brief suggested four phases. I split its Phase 4 into 4 and 5 because lifecycle (destructive, needs grace periods and purges) and policies/parity (additive, low risk) are independent, and one phase holding both would be too big to review.

---

## Phase 1: Foundation

**Goal.** People can create an organization, invite members and guests by email, organize them into teams with team roles, and switch between Personal and the organization. Team spaces exist but hold no content yet. Every membership, team and policy change is audited from the first release.

**Scope.**
- Organizations (name, default team role policy) and their Owner, Admin, Member and Guest roles, with the last-owner rule enforced under a row lock.
- A default team ("General", renamable, cannot be deleted) created with each organization. Every Member and Admin joins it with the organization's default team role (Editor, or Viewer by policy). Guests never join it automatically.
- Teams: create and rename (Owners and Admins), team members with Maintainer, Editor and Viewer roles, managed by Maintainers. Owners and Admins act as Maintainer everywhere without being listed.
- Email invites: an address, an organization role, and teams. They reuse the part C and E rules: an invite waits for that address to be *verified* on an account (no claim tokens), so with mail off only accounts an admin created or marked verified can accept. One `organization_invite` email per invite, counted on the inviter's existing 20-per-hour invite budget (`invite_email_limit`, `AppState::invite_limiter`).
- Leaving and removing members (access ends at once; there is no content yet). The last owner cannot leave or be demoted.
- The audit log table `organization_audit_events`, written in the same transaction as each change, append-only (a trigger refuses `UPDATE`), and purged hourly after 1 year. Organization settings → Audit log shows the latest 100 entries.
- The Plans seam: `linkdean_core::plans::Plans` with only `SelfHosted` (everything allowed) and one API helper, `crate::plans::require`, called before creating an organization, accepting an invite and creating a team. Self-hosted never runs a query for it.
- Instance setting `platform_settings.organization_creation`: `anyone` (default) or `instance_admins`.
- Web: "Create an organization" and "Organization invites" in the account menu; a workspace switcher at the top of the sidebar for members of an organization; an organization sidebar listing the user's teams (default first); empty team space pages; Organization settings (Members, Teams, Policies, Audit log); a subtly tinted header inside an organization. The URL is the source of truth for the workspace: `/orgs/{id}/…` is the organization, anything else is Personal.

**Data model.** Migration `045_organizations.sql`: `organizations`, `organization_members`, `teams`, `team_members`, `organization_invites`, `organization_invite_teams`, `organization_audit_events`, and `platform_settings.organization_creation`. No existing table changes apart from that one column.

**Risks.**
- *Last-owner races:* two owners demoting each other at once. Every membership change locks the organization row (`SELECT … FOR UPDATE`) and counts owners inside the transaction. A concurrency test covers it.
- *Invite address matching:* matching is by `lower(email)` against the account's *current, verified* address, at read time. An email change to a new verified address picks up its invites; an unverified account sees none (only a count, to prompt verification).
- *Directory leaks to Guests:* every list endpoint checks the role. A non-member gets `404`, not `403`, so the API never confirms that an organization exists.
- *Audit gaps:* audit writes share the transaction with the change, so a change cannot commit without its event.

**Defers.** Any content in team spaces (Phase 2); archive and delete of teams and organizations, ownership transfer (Phase 4); policies beyond the default team role, audit filters and CSV, the organization logo (Phase 5); invite links, verified domains, SSO and SCIM (backlog).

---

## Phase 2: Team spaces (the ownership refactor)

**Goal.** Folders, bookmarks, tags and dashboards can be owned by an organization and live in a team space, with access from team roles. Personal behaves exactly as before.

Phase 2 is the largest phase. Ship it in two halves, each green and released on its own:

- **2A: Team folders, bookmarks and tags.** Ownership columns, permission helpers, team space library pages, search scoping, Deleted items for team content, team activity recorded with `team_id`.
- **2B: Dashboards, moves and capture.** Team dashboards; moving personal → team (folder subtree with its bookmarks, after the warning); copying team → personal (always allowed until the Phase 5 policy); the capture picker (paste-to-add saves to the current workspace); the "all my workspaces" search toggle.

**Data model (2A).** On `categories`, `bookmarks`, `tags` (2A) and `dashboards` (2B):

```sql
ALTER TABLE categories ALTER COLUMN owner_user_id DROP NOT NULL;
ALTER TABLE categories
    ADD COLUMN organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    ADD COLUMN team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    ADD COLUMN created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD CONSTRAINT categories_one_owner CHECK (
        (owner_user_id IS NOT NULL AND organization_id IS NULL AND team_id IS NULL)
        OR (owner_user_id IS NULL AND organization_id IS NOT NULL AND team_id IS NOT NULL)
    );
CREATE INDEX categories_team_idx ON categories (team_id, deleted_at) WHERE team_id IS NOT NULL;
-- same for bookmarks, tags (UNIQUE (team_id, name) WHERE team_id IS NOT NULL), dashboards
```

Organization rows carry `owner_user_id = NULL`, so **every existing `owner_user_id = $user` query excludes them by construction**. That is the main safety property of this design. A team row cannot be vault content (`CHECK (team_id IS NULL OR NOT is_vault)`), cannot be hidden (`is_hidden` is a per-owner declutter flag), and stays `visibility = 'private'` with no `public_slug` until the Phase 5 policy. `activity_events` gets a nullable `team_id`.

**Permission checks.** In core, `ContentOwner { User(Uuid), Team { organization_id, team_id } }` and a `fn owner(&self) -> ContentOwner` on `Category`, `Bookmark` and `Dashboard`. `owner_user_id: Uuid` becomes `Option<Uuid>` in the models, so the compiler lists every comparison for review. `category_access` gains a branch for `team_id IS NOT NULL` *before* the share walk: effective team role (Phase 1's `effective_team_role`), mapped Maintainer → `CategoryAccess::Manage` (new), Editor → `Write`, Viewer → `Read`. `bookmark_can_read` and `bookmark_can_write` gain the same branch on the bookmark's own `team_id`. A team bookmark may only sit in folders of its own team space (`BadRequest` otherwise). Bookmarks added to a team folder are owned by the organization, with `created_by_user_id` set to the adder, unlike personal shared folders.

The personal sharing code never handles organization rows: `create_share`, dashboard shares, public links, forks and vault moves refuse team content with `BadRequest`. Social routes (`social.rs`) refuse team content until Phase 5.

**Search.** Index documents gain `team_id` (filterable). `meili_visibility_filter` grows an `OR team_id IN [readable teams]` term, and a `workspace` parameter (`personal`, `org:<id>`, `all`) narrows it. The Postgres recheck stays the final gate. Upgrading needs `ensure_index` (new filterable attribute) plus a reindex (`POST /api/admin/search/reindex`), noted in SELF_HOSTED.

**Deleted items.** Team content trashed by an Editor or above goes to the team's Deleted items. Editors and above see and restore it, and Maintainers purge it. `purge_expired_trash` must purge organization rows after a fixed 30 days (the Phase 5 retention policy makes it configurable). Today it joins `user_settings` by owner, so without this change organization rows would never be purged.

**Risks.** See "The riskiest change" below. Also:
- multi-folder bookmarks crossing owners;
- Meilisearch documents indexed before the reindex lack `team_id` (harmless: team rows did not exist before);
- the dashboard default index (`dashboards_one_default_per_user`) must not apply to team rows (NULLs are distinct, so it does not);
- `ON DELETE CASCADE` from `owner_user_id` no longer protects organization rows, which is intended: removing a member must not delete organization content, so the column is `created_by_user_id … ON DELETE SET NULL`.

**Defers.** Restricted folders (3), team lifecycle (4), policies, social on team content, import/export, library health (5).

### The riskiest change

Phase 2 touches every permission check: about 220 `owner_user_id` mentions in 16 repo files, and every route that calls `require_category_*`, `bookmark_can_*` or `require_dashboard_owner`. A missed site is either an IDOR (organization content readable by a non-member) or a personal regression. Here is how to keep personal behavior unchanged and proven:

1. **NULL owner for organization rows.** Existing personal filters (`owner_user_id = $1`) cannot match organization rows, so a missed site fails closed: team content is invisible on it, not exposed.
2. **Type change as a checklist.** `owner_user_id: Option<Uuid>` in `Category`, `BookmarkView`, `Tag` and `Dashboard` makes every Rust comparison a compile error. Each fix keeps the personal arm byte-for-byte the same as today, and team access goes in a separate arm. Raw SQL is not caught by the compiler, so the 2A plan lists every SQL site per file (from `grep -n owner_user_id crates/db/src/repo/*.rs`) as a review table, with a "personal unchanged / team arm / refuses team" column.
3. **Existing authz tests are frozen.** `crates/db/tests/authz.rs`, `share_invites.rs`, `shared_trash.rs`, `shared_bookmark_links.rs`, `vault_moves.rs`, `blocks.rs`, `social_nested.rs`, `export_import.rs` and `trash_retention.rs` must pass with no edits beyond compile fixes (`Some(user.id)` where a field became optional). Any behavioral edit to them is a stop-and-ask.
4. **New matrix tests.** `crates/db/tests/team_authz.rs` covers role (Owner, Admin, Maintainer, Editor, Viewer, Guest in the team, Guest not in it, Member not in the team, removed member, other organization, no organization) × action (read folder, list folder, create bookmark, edit, move, trash, restore, purge, tag, search hit, dashboard read and edit). The same file also tests personal isolation: an organization Owner cannot read a member's personal folder, bookmark, tag, dashboard, search hit or Deleted items.
5. **Refuse, don't adapt.** Personal sharing, public links, forks, vault moves and social refuse team content in Phase 2, so their code paths stay personal-only and keep their tests.
6. **Search recheck stays.** Even if a filter is wrong, `bookmark_can_read` re-checks every hit.

---

## Phase 3: Restricted folders

**Goal.** A Maintainer (or an Owner or Admin) can restrict a team folder to some members or teams of its space. Restrictions only narrow access. Owners and Admins always see restricted folders.

**Scope.** `folder_restrictions (category_id, user_id NULL, team_id NULL, CHECK exactly one)`. A folder is restricted when it has any rows, and the restriction applies to its descendants. Effective access is the team role, narrowed by the nearest restricted ancestor; this is computed in `category_access`'s team arm while walking up the parent chain. Restricting and unrestricting are audited (`folder_restricted`, `folder_unrestricted`). The sidebar and explorer show a lock. Search: documents gain `restricted_root_ids`, and the filter excludes restricted roots the user cannot read (`NOT restricted_root_ids IN [...]`); the Postgres recheck stays. Activity events in a restricted folder are visible only to its readers.

**Risks.** Bookmarks in several folders where one is restricted: readable when *any* readable folder holds it, as today's multi-folder rule does. A bookmark whose *only* folder is restricted is unreadable to others. Moves into or out of a restricted folder change access, so moves are audited.

**Defers.** Audit of restricted-folder *views* (backlog).

---

## Phase 4: Lifecycle and instance admin

**Goal.** Teams and organizations can end safely, people can leave without breaking content, and instance admins can run organizations without seeing their content.

**Scope.**
- Teams: archive (read-only, hidden from the switcher, restorable) by Maintainers and above; delete (Owners and Admins only; never the default team): content goes to the organization's Deleted items for the retention period, then is purged. `teams.archived_at`, `teams.deleted_at`.
- Organization deletion: Owners only, confirmed by typing the name; `organizations.deleted_at` with a 30-day grace and restore, then hard purge (maintenance), with an email to every owner (`organization_deleted` template) and export by Admins during the grace period. Danger zone tab.
- Ownership transfer (promote another owner, then leave).
- Offboarding: content stays, attributed via `created_by_user_id`; a creator who is no longer a member shows as "Former member" (computed at read time from `organization_members`).
- Account deletion (if and when it is built): treated as leaving every organization, and blocked while the account is the last owner of one. The guard lives in a `db::assert_can_delete_account` that the future route must call.
- Instance admin: an Admin → Organizations panel with name, member count, storage (bookmark count and preview bytes) and status; suspend (read-only; members cannot open it; "Suspended" shown) and delete, for abuse or non-payment. `organizations.suspended_at`, checked in the organization access helpers. Instance admins never get content endpoints.
- Instance-wide ban: organization member lists show "Suspended by instance". Phase 1 already returns the flag; Phase 4 adds the copy and the "remove anyway" path.

**Risks.** Destructive purges: run them only from maintenance, behind `deleted_at < now() - interval '30 days'`, with tests at the boundary. A suspended organization must be checked in *every* team access path, which works because Phase 2 routes all of them through one helper.

**Defers.** Billing, and the seat counts the SaaS needs.

---

## Phase 5: Policies, audit and parity

**Goal.** Owners and Admins control the organization's policies, read and export the full audit log, and team spaces reach the spec's v1 feature list.

**Scope.**
- Policies (`organization_policies` columns on `organizations`): public sharing allowed (off by default; turning it on lets Admins enable public links on team folders, which moderation then covers in `exposed_content`); who creates teams (Owners and Admins, or every Member); team → personal copy (on by default); Deleted items retention (default 30 days). Each change is audited (`policy_changed`).
- Audit log: filters (person, action, date range), keyset pagination, CSV export (`text/csv`, streamed), Business-plan gate through `crate::plans::require` (a no-op on self-hosted).
- Team activity feed: `GET /api/teams/{id}/activity` from `activity_events.team_id`.
- Library health per space (broken links, duplicates) run by Maintainers; import and export by Maintainers, into and out of a team space.
- Likes, comments and ratings on team content, visible to the team's readers.
- Organization logo, using the avatar pipeline (crop, 256×256 WebP), shown in the switcher.

**Risks.** Turning public sharing on widens exposure. The policy is off by default, only Admins can turn it on, and the change is audited.

**Defers.** Everything in the spec's Backlog section.

---

## Product owner answers (2026-11-04)

These override any recommendation in this document that says otherwise.

| # | Answer |
| --- | --- |
| 1 | Invites expire after 30 days; resend renews. As recommended. |
| 2 | Only Owners grant Admin. As recommended. |
| 3 | Members can join a team themselves only if they can see it and it is **open to joiners**. Each team has an "Open to joiners" setting that Maintainers change (default on; the default team is always joinable in effect, since members are on it). Members see the teams they are listed in plus open teams; closed teams they are not in are hidden from them (Owners and Admins see every team). Guests never join by themselves. |
| 4 | Nobody but Guests leaves the default team; role change only. As recommended. |
| 5 | The default team is named **"Default team"**, renamable. |
| 6 | An invite to an address with no account while instance registration is closed must not fail silently. The invite is still saved, the invite response says `invitee_can_register: false`, and the invite dialog warns: "Registration is closed on this instance, so they can't create an account yet. Ask an instance admin to create one or open registration." The invite email to such an address says "Ask {{ inviter }} or your Linkdean admin for an account with this address" instead of "Create an account". Opening registration later lets the invite be accepted as normal. |
| 7 | The invite email ignores the "Share invites" preference and counts on the inviter's 20-per-hour budget. As recommended. |
| 8 | Members see names and roles; only Owners and Admins see emails. As recommended. |
| 9 | Bookmarks added to a team folder are owned by the organization **and keep the user who created them** (`created_by_user_id`), shown as their author. |
| 10 | Moving a shared personal folder into a team ends its shares after a warning; invitees are notified. As recommended. |
| 11 | Editors and up see and restore a team's Deleted items; Maintainers, Admins and Owners purge. As recommended. |
| 12 | Team → personal copy includes bookmarks and folder subtrees (title, URL, notes, tags), not dashboards or likes, comments and ratings. As recommended. |
| 13 | Purging an organization also deletes its audit log; the server log records the purge. As recommended. |

## Open questions for the product owner (answered above)

These must be answered before the phase named. Each has my recommendation; Phase 1's plan assumes the recommendations for 1–8, so a different answer means editing that plan first.

| # | Before | Question | Recommendation |
| --- | --- | --- | --- |
| 1 | Phase 1 | Do invites expire? | Yes, after 30 days. Admins can resend, which renews the invite and sends a new email. |
| 2 | Phase 1 | Who can grant Admin, by invite or promotion? | Owners only. Admins invite and manage Members and Guests. This matches "Owner: promote admins". |
| 3 | Phase 1 | "Join or be added to teams": can Members join a team themselves? | Yes. Members see every team's name and can join any team, taking the organization's default team role. Guests cannot. Closed teams go to the backlog. |
| 4 | Phase 1 | Can a Member or Admin leave, or be removed from, the default team? | No. The default team is "everyone"; only their role in it can change. Leaving the organization removes them. |
| 5 | Phase 1 | What is the default team called? | "General", renamable by Owners and Admins. |
| 6 | Phase 1 | Does an organization invite let someone register while instance registration is closed? | Not in v1. The invite waits, and the organization's invite list shows "No account yet". The instance admin still controls sign-ups (registration invites). Revisit for the SaaS. |
| 7 | Phase 1 | Is the invite email sent even when the invitee turned off "Share invites" email? | Yes. It is a one-off invitation to join an organization, not a notification. It counts against the inviter's 20-per-hour invite email budget. |
| 8 | Phase 1 | Can Members (not Guests) see the member directory? | Yes, names and roles. Emails are shown to Owners and Admins only. |
| 9 | Phase 2 | Bookmarks added to a team folder belong to the organization, unlike personal shared folders where the adder owns them. Confirm. | Yes. Otherwise content would leave with the person. |
| 10 | Phase 2 | Moving a personal folder that is shared with people into a team space: refuse, or end those shares? | End them in the same move, after the warning lists who loses access; invitees get the existing "share revoked" notification. |
| 11 | Phase 2 | Who sees and restores a team's Deleted items? | Editors and above see and restore; Maintainers, Admins and Owners purge. |
| 12 | Phase 2 | What does "copy team → personal" copy? | Bookmarks and folder subtrees (title, URL, notes, tags). Not dashboards, likes, comments or ratings. |
| 13 | Phase 4 | When an organization is purged, does its audit log go too? | Yes. Nobody could view it. The server log records the purge (organization id, name, who deleted it). |
