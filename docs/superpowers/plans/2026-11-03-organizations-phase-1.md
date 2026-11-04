# Organizations Phase 1 (Foundation) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** People can create an Organization, invite Members and Guests by email, put them in Teams with team roles, and switch between the Personal workspace and an organization. Team spaces exist but are empty. Every membership, team and policy change is in the audit log from the first release.

**Architecture:** New tables only (`organizations`, `organization_members`, `teams`, `team_members`, `organization_invites`, `organization_invite_teams`, `organization_audit_events`) plus one platform setting. Composite foreign keys make "a team member is a member of the team's organization" a database rule, and removing an organization member cascades out of its teams. Role rules are pure functions in `linkdean-core` (unit-tested). Every membership change runs in one transaction that locks the organization row and writes its audit event, so a change cannot commit without its event. Invites match an account's current *verified* address when it is read, so no claim tokens are needed. The Plans seam is `linkdean_core::plans::Plans` (only `SelfHosted`, which allows everything), reached through one API helper. In the web app, the URL is the workspace: `/orgs/{id}/…` is the organization, everything else is Personal.

**Tech Stack:** Rust (axum 0.8, sqlx 0.8), `linkdean-mail` (MJML), React + Mantine 9 + TanStack Router/Query + zod, vitest.

Spec: [`docs/superpowers/specs/2026-11-03-organizations-and-teams-design.md`](../specs/2026-11-03-organizations-and-teams-design.md). Phases: [`2026-11-03-organizations-and-teams-phases.md`](./2026-11-03-organizations-and-teams-phases.md). **The product owner's answers in the phases document ("Product owner answers (2026-11-04)") override this plan wherever they differ.** In particular: the default team is named "Default team" (not "General"); teams have an "Open to joiners" setting (`open_to_join BOOLEAN NOT NULL DEFAULT true` on teams, changed by Maintainers, audited) and Members see and can join only open teams or teams they are listed in; and an invite to an address with no account while registration is closed is saved but flagged (`invitee_can_register: false`), warned about in the invite dialog, and its email asks the invitee to get an account from the inviter or an admin. Every task below that mentions "General", unrestricted joining, or invites without that flag must apply these answers.

Patterns to copy:
- Pending invites and email: `crates/db/src/repo/pending_shares.rs` (`normalize_email`, upsert on a partial unique index), `crates/api/src/notification_mail.rs` (`InviteEmail`, `with_invite_email`, `invite_clock`, `owner_label`, the `invite_email_limit` budget).
- Platform settings: `crates/db/src/repo/platform.rs`, `routes/admin/mod.rs::patch_platform_settings`.
- Templates: `crates/mail/templates/share_invite_pending.*`, registered in `crates/mail/src/render.rs` `TEMPLATES`.
- DB tests: `crates/db/tests/email_verification.rs` (`pool_or_skip`, unique addresses, `db::mark_email_verified`).
- Web: `pages/settings/NotificationEmailsPanel.tsx` and its test (mocking `@/api`), `hooks/share-invites/useShareInvites.ts`.

**Repo rules for every task:**

- Work on `main`. Commit each green task and push. Never commit red. Commit messages are normal prose ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Stage only the files you changed; never `git add -A` (`brand/graphics`, `graphify-out/` and anything else you did not touch, such as `docs/IDEAS.md`, stay out).
- Rust: always `cargo … -j 2`. Tests use only `DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test` (the throwaway `pg-check` container). Check the `test result` counts and that no `skipping` line printed. Smoke runs use the `smoke` database on the same server. Never the dev database; never touch containers named `linkdean-*`.
- Rust format: `docker run --rm -v "$PWD":/w -w /w rust:1.89 sh -c "rustup component add rustfmt >/dev/null 2>&1; cargo fmt --all -- --check"`. Clippy: `cargo clippy -j 2 --workspace --all-targets -- -D warnings`.
- Web, from `apps/web`: `npx vitest run`, `npx prettier --check src`, `npx oxlint src`, `npx tsc -b --noEmit`. `routeTree.gen.ts` is tracked: regenerate with `npx tsr generate` after adding routes, and commit it with them (skill `linkdean-web-add-route`).
- Sidebar, header and phone tab changes follow skill `linkdean-web-navigation`: one control per action per width.
- American spelling. Use the spec's words exactly in copy and identifiers: Organization, Workspace, Personal, Team, Team space, Members, Guests, Invite, Owner/Admin/Member/Guest, Maintainer/Editor/Viewer, Instance admin, Plans.
- Before committing a migration, check the next free number on `main` (`ls migrations`); `045` is free at commit `721ca46`.
- After changing files, run `graphify update .` from the repo root.
- Every name here was checked against commit `721ca46`. Verify each signature before you use it, adapt, and report deviations.

---

## File map

| File | Responsibility |
| --- | --- |
| `migrations/045_organizations.sql` (create) | All Phase 1 tables, the append-only trigger, `platform_settings.organization_creation` |
| `crates/core/src/models/organizations.rs` (create), `models/mod.rs` | Roles, permission rules, audit actions, views and request bodies |
| `crates/core/src/plans.rs` (create), `lib.rs` | The Plans seam |
| `crates/core/src/error/mod.rs` | `Error::PlanLimit` |
| `crates/core/src/models/mod.rs` (`PlatformSettings`, `UpdatePlatformSettings`) | `organization_creation` |
| `crates/db/src/repo/organizations/{mod,access,audit,organizations,members,teams,invites}.rs` (create), `repo/mod.rs` | Persistence and access checks |
| `crates/db/src/repo/platform.rs` | Read and set `organization_creation` |
| `crates/db/tests/organizations.rs`, `organization_teams.rs`, `organization_invites.rs`, `organization_audit.rs` (create) | Tests |
| `crates/mail/templates/organization_invite.{mjml,subject,sample.json}` (create), `crates/mail/src/render.rs` | Invite email |
| `crates/api/src/routes/organizations/{mod,members,teams,invites}.rs` (create), `routes/mod.rs` | Routes |
| `crates/api/src/plans.rs`, `crates/api/src/organization_mail.rs` (create), `main.rs`, `state/mod.rs`, `error/mod.rs`, `maintenance.rs`, `routes/admin/mod.rs` | Plans seam, invite email, wiring, audit purge, instance setting |
| `apps/web/src/schemas/organizations/`, `api/api.ts`, `hooks/organizations/`, `services/workspace/` (create) | Contract, data hooks, workspace from the URL |
| `apps/web/src/pages/app-shell/{WorkspaceSwitcher,OrganizationSidebar}.tsx` (create), `AppSidebar.tsx`, `AppShellHeader.tsx`, `AppShell.tsx`, `AppShell.module.css` | Switcher, organization sidebar, account menu, header tint |
| `apps/web/src/components/organization/CreateOrganizationModal.tsx` (create) | Create flow |
| `apps/web/src/pages/organization/` (create), `pages/organization-invites/` (create), `pages/index.ts` | Organization home, team space, settings tabs, invites page |
| `apps/web/src/routes/_app/orgs.$orgId.index.tsx`, `orgs.$orgId.settings.tsx`, `orgs.$orgId.teams.$teamId.tsx`, `invites.tsx` (create), `routeTree.gen.ts` | Routes |
| `apps/web/src/pages/settings/AdminPlatformSettings.tsx` | "Who can create organizations" |
| Docs | ARCHITECTURE, FEATURES, SELF_HOSTED, ROADMAP item 12 |

---

### Task 1: Core domain (roles, rules, Plans seam)

**Files:**
- Create: `crates/core/src/models/organizations.rs`, `crates/core/src/plans.rs`
- Modify: `crates/core/src/models/mod.rs` (add `mod organizations; pub use organizations::*;` next to `mod moderation;`), `crates/core/src/lib.rs` (`pub mod plans;`), `crates/core/src/error/mod.rs`

- [ ] **Step 1: Write the failing unit tests** at the bottom of `organizations.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_round_trip() {
        for r in [OrgRole::Owner, OrgRole::Admin, OrgRole::Member, OrgRole::Guest] {
            assert_eq!(OrgRole::parse(r.as_str()), Ok(r));
        }
        for r in [TeamRole::Maintainer, TeamRole::Editor, TeamRole::Viewer] {
            assert_eq!(TeamRole::parse(r.as_str()), Ok(r));
        }
        assert!(OrgRole::parse("root").is_err());
    }

    #[test]
    fn owners_and_admins_act_as_maintainer_everywhere() {
        assert_eq!(effective_team_role(OrgRole::Owner, None), Some(TeamRole::Maintainer));
        assert_eq!(effective_team_role(OrgRole::Admin, Some(TeamRole::Viewer)), Some(TeamRole::Maintainer));
        assert_eq!(effective_team_role(OrgRole::Member, Some(TeamRole::Viewer)), Some(TeamRole::Viewer));
        assert_eq!(effective_team_role(OrgRole::Member, None), None);
        assert_eq!(effective_team_role(OrgRole::Guest, None), None);
    }

    #[test]
    fn guests_cannot_see_the_directory_or_manage_anything() {
        for a in OrgAction::ALL {
            assert!(!org_allows(OrgRole::Guest, a), "{a:?}");
        }
        assert!(org_allows(OrgRole::Member, OrgAction::ViewDirectory));
        assert!(!org_allows(OrgRole::Member, OrgAction::InviteMembers));
        assert!(org_allows(OrgRole::Admin, OrgAction::InviteMembers));
        assert!(!org_allows(OrgRole::Admin, OrgAction::GrantAdmin));
        assert!(org_allows(OrgRole::Owner, OrgAction::GrantAdmin));
    }

    #[test]
    fn only_owners_grant_or_take_admin_and_owner() {
        // Admin may move people between Member and Guest only.
        assert!(check_role_change(OrgRole::Admin, OrgRole::Member, OrgRole::Guest, 1).is_ok());
        assert!(matches!(check_role_change(OrgRole::Admin, OrgRole::Member, OrgRole::Admin, 1), Err(Error::Forbidden)));
        assert!(matches!(check_role_change(OrgRole::Admin, OrgRole::Admin, OrgRole::Member, 1), Err(Error::Forbidden)));
        assert!(matches!(check_role_change(OrgRole::Admin, OrgRole::Owner, OrgRole::Member, 2), Err(Error::Forbidden)));
        assert!(check_role_change(OrgRole::Owner, OrgRole::Member, OrgRole::Owner, 1).is_ok());
    }

    #[test]
    fn the_last_owner_cannot_be_demoted_or_removed_or_leave() {
        // `other_owners` counts owners other than the target.
        assert!(matches!(check_role_change(OrgRole::Owner, OrgRole::Owner, OrgRole::Admin, 0), Err(Error::Conflict(_))));
        assert!(check_role_change(OrgRole::Owner, OrgRole::Owner, OrgRole::Admin, 1).is_ok());
        assert!(matches!(check_leave(OrgRole::Owner, 0), Err(Error::Conflict(_))));
        assert!(check_leave(OrgRole::Owner, 1).is_ok());
        assert!(check_leave(OrgRole::Guest, 0).is_ok());
    }

    #[test]
    fn removal_follows_the_same_ladder() {
        assert!(check_removal(OrgRole::Admin, OrgRole::Member).is_ok());
        assert!(check_removal(OrgRole::Admin, OrgRole::Guest).is_ok());
        assert!(matches!(check_removal(OrgRole::Admin, OrgRole::Admin), Err(Error::Forbidden)));
        assert!(matches!(check_removal(OrgRole::Admin, OrgRole::Owner), Err(Error::Forbidden)));
        assert!(check_removal(OrgRole::Owner, OrgRole::Admin).is_ok());
        assert!(matches!(check_removal(OrgRole::Member, OrgRole::Guest), Err(Error::Forbidden)));
    }

    #[test]
    fn invites_never_grant_owner_and_admin_only_from_owners() {
        assert!(matches!(check_invite_role(OrgRole::Owner, OrgRole::Owner), Err(Error::BadRequest(_))));
        assert!(check_invite_role(OrgRole::Owner, OrgRole::Admin).is_ok());
        assert!(matches!(check_invite_role(OrgRole::Admin, OrgRole::Admin), Err(Error::Forbidden)));
        assert!(check_invite_role(OrgRole::Admin, OrgRole::Guest).is_ok());
        assert!(matches!(check_invite_role(OrgRole::Member, OrgRole::Member), Err(Error::Forbidden)));
    }

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(organization_name("  Acme Inc.  ").unwrap(), "Acme Inc.");
        assert!(organization_name("   ").is_err());
        assert!(organization_name(&"x".repeat(81)).is_err());
    }
}
```

- [ ] **Step 2: Run, expect compile failure:** `cargo test -j 2 -p linkdean-core organizations` → errors such as "cannot find type `OrgRole`".

- [ ] **Step 3: Implement** `organizations.rs`:

```rust
//! Organizations and teams: roles, permission rules, audit actions, views.
//! The rules are pure so they can be tested without a database; `linkdean-db` calls them inside
//! the transaction that changes membership.

use crate::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Longest organization or team name, in characters.
pub const NAME_MAX: usize = 80;
/// What a new organization's default team is called.
pub const DEFAULT_TEAM_NAME: &str = "General";
/// How long an invite stays open.
pub const INVITE_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole { Owner, Admin, Member, Guest }

impl OrgRole {
    pub fn as_str(self) -> &'static str {
        match self { Self::Owner => "owner", Self::Admin => "admin", Self::Member => "member", Self::Guest => "guest" }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "owner" => Ok(Self::Owner), "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member), "guest" => Ok(Self::Guest),
            _ => Err(format!("unknown organization role: {s}")),
        }
    }
    /// Owners and Admins: act as Maintainer in every team, see every team space.
    pub fn manages(self) -> bool { matches!(self, Self::Owner | Self::Admin) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Ordered: Viewer < Editor < Maintainer.
pub enum TeamRole { Viewer, Editor, Maintainer }

impl TeamRole {
    pub fn as_str(self) -> &'static str {
        match self { Self::Maintainer => "maintainer", Self::Editor => "editor", Self::Viewer => "viewer" }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "maintainer" => Ok(Self::Maintainer), "editor" => Ok(Self::Editor), "viewer" => Ok(Self::Viewer),
            _ => Err(format!("unknown team role: {s}")),
        }
    }
}

/// The role that counts in a team: Owners and Admins are Maintainer without being listed.
pub fn effective_team_role(org: OrgRole, listed: Option<TeamRole>) -> Option<TeamRole> {
    if org.manages() { Some(TeamRole::Maintainer) } else { listed }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgAction {
    /// See the member list (names and roles).
    ViewDirectory,
    /// See member email addresses.
    ViewMemberEmails,
    InviteMembers,
    /// Invite as Admin, promote to or demote from Admin or Owner.
    GrantAdmin,
    RemoveMembers,
    ManageTeams,
    EditSettings,
    ViewAuditLog,
}

impl OrgAction {
    pub const ALL: [Self; 8] = [
        Self::ViewDirectory, Self::ViewMemberEmails, Self::InviteMembers, Self::GrantAdmin,
        Self::RemoveMembers, Self::ManageTeams, Self::EditSettings, Self::ViewAuditLog,
    ];
}

pub fn org_allows(role: OrgRole, action: OrgAction) -> bool {
    use OrgAction::*;
    match role {
        OrgRole::Owner => true,
        OrgRole::Admin => !matches!(action, GrantAdmin),
        OrgRole::Member => matches!(action, ViewDirectory),
        OrgRole::Guest => false,
    }
}

/// `other_owners`: owners other than the target, counted under the organization row lock.
pub fn check_role_change(actor: OrgRole, target: OrgRole, new: OrgRole, other_owners: i64) -> Result<(), Error> {
    let touches_admin = |r: OrgRole| r.manages();
    if touches_admin(target) || touches_admin(new) {
        if !org_allows(actor, OrgAction::GrantAdmin) { return Err(Error::Forbidden); }
    } else if !org_allows(actor, OrgAction::RemoveMembers) {
        return Err(Error::Forbidden);
    }
    if target == OrgRole::Owner && new != OrgRole::Owner && other_owners == 0 {
        return Err(Error::Conflict("make someone else an owner first".into()));
    }
    Ok(())
}

pub fn check_removal(actor: OrgRole, target: OrgRole) -> Result<(), Error> {
    if !org_allows(actor, OrgAction::RemoveMembers) { return Err(Error::Forbidden); }
    if target.manages() && !org_allows(actor, OrgAction::GrantAdmin) { return Err(Error::Forbidden); }
    Ok(())
}

/// Leaving: the last owner must hand over first. (Removal needs no owner count: only an owner can
/// remove an owner, and nobody removes themselves, so another owner always remains.)
pub fn check_leave(role: OrgRole, other_owners: i64) -> Result<(), Error> {
    if role == OrgRole::Owner && other_owners == 0 {
        return Err(Error::Conflict("make someone else an owner before you leave".into()));
    }
    Ok(())
}

pub fn check_invite_role(actor: OrgRole, role: OrgRole) -> Result<(), Error> {
    if role == OrgRole::Owner {
        return Err(Error::BadRequest("invite as an admin, then make them an owner".into()));
    }
    if !org_allows(actor, OrgAction::InviteMembers) { return Err(Error::Forbidden); }
    if role == OrgRole::Admin && !org_allows(actor, OrgAction::GrantAdmin) { return Err(Error::Forbidden); }
    Ok(())
}

/// Trimmed name, 1..=NAME_MAX characters. Used for organizations and teams.
pub fn organization_name(raw: &str) -> Result<String, Error> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(Error::BadRequest(format!("name must be 1 to {NAME_MAX} characters")));
    }
    Ok(name.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    OrganizationCreated, OrganizationRenamed, PolicyChanged,
    MemberInvited, InviteResent, InviteRevoked, InviteAccepted, InviteDeclined,
    MemberRoleChanged, MemberRemoved, MemberLeft,
    TeamCreated, TeamRenamed, TeamMemberAdded, TeamMemberRemoved, TeamRoleChanged,
}
// as_str/parse: the snake_case names above, exactly as in migration 045's CHECK.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditTarget { Organization, Member, Invite, Team }
// as_str/parse likewise.
```

Add `AuditAction::as_str/parse` and `AuditTarget::as_str/parse` (write each match out, and add a round-trip test over every variant like `roles_round_trip`).

Views and bodies in the same file (all `Serialize`; bodies `Deserialize`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization { pub id: Uuid, pub name: String, pub default_team_role: TeamRole, pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }

/// One row of the workspace switcher.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationSummary { pub id: Uuid, pub name: String, pub role: OrgRole, pub default_team_id: Uuid }

#[derive(Debug, Clone, Serialize)]
pub struct TeamView {
    pub id: Uuid, pub organization_id: Uuid, pub name: String, pub is_default: bool,
    /// The viewer's listed role, if listed.
    pub my_role: Option<TeamRole>,
    /// What the viewer can do: `effective_team_role`.
    pub effective_role: Option<TeamRole>,
    pub member_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrganizationView { pub organization: Organization, pub role: OrgRole, pub teams: Vec<TeamView> }

#[derive(Debug, Clone, Serialize)]
pub struct OrganizationMemberView {
    pub user_id: Uuid, pub display_name: String,
    /// Only for Owners and Admins (`OrgAction::ViewMemberEmails`).
    pub email: Option<String>,
    pub role: OrgRole, pub joined_at: DateTime<Utc>,
    /// An instance admin banned this account; it keeps its memberships.
    pub suspended_by_instance: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamMemberView { pub user_id: Uuid, pub display_name: String, pub role: TeamRole, pub org_role: OrgRole }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteTeam { pub team_id: Uuid, pub role: TeamRole }

/// An invite as Owners and Admins see it.
#[derive(Debug, Clone, Serialize)]
pub struct OrganizationInviteView {
    pub id: Uuid, pub organization_id: Uuid, pub invitee_email: String, pub role: OrgRole,
    pub teams: Vec<InviteTeam>, pub invited_by_label: String,
    pub created_at: DateTime<Utc>, pub expires_at: DateTime<Utc>,
    /// Whether an account with a verified address can accept it now.
    pub can_accept_now: bool,
}

/// An invite as the invitee sees it.
#[derive(Debug, Clone, Serialize)]
pub struct MyOrganizationInvite {
    pub id: Uuid, pub organization_id: Uuid, pub organization_name: String, pub role: OrgRole,
    pub invited_by_label: String, pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MyOrganizationInvites {
    pub invites: Vec<MyOrganizationInvite>,
    /// Open invites to this account's address that it cannot see until it verifies the address.
    pub waiting_for_verification: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEventView {
    pub id: Uuid, pub actor_user_id: Option<Uuid>, pub actor_label: String,
    pub action: AuditAction, pub target_type: AuditTarget, pub target_id: Option<Uuid>,
    pub target_label: String, pub details: serde_json::Value, pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrganization { pub name: String }

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateOrganization { pub name: Option<String>, pub default_team_role: Option<TeamRole> }

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrganizationInvite { pub email: String, pub role: OrgRole, #[serde(default)] pub teams: Vec<InviteTeam> }

/// Instance setting: who may create organizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationCreation { Anyone, InstanceAdmins }
// as_str/parse: "anyone", "instance_admins".
```

`default_team_role` must be Editor or Viewer: `update_organization` refuses `Maintainer` with `BadRequest("the default team role is editor or viewer")`. Add a unit test `default_team_role_is_editor_or_viewer` around a small `pub fn check_default_team_role(r: TeamRole) -> Result<(), Error>`.

- [ ] **Step 4: Plans seam** `crates/core/src/plans.rs`:

```rust
//! The one place SaaS plan limits are decided. Self-hosted has every feature, so
//! `Plans::SelfHosted` allows everything and callers skip counting usage. A SaaS build adds
//! variants here; no other code checks plans.

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanFeature { CreateOrganization, AddMember, AddGuest, CreateTeam }

/// What the organization (or the user, for `CreateOrganization`) uses now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanUsage { pub organizations_owned: i64, pub members: i64, pub guests: i64, pub teams: i64 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plans { SelfHosted }

impl Plans {
    /// When false, callers need not count usage.
    pub fn is_gated(self) -> bool {
        match self { Self::SelfHosted => false }
    }

    pub fn check(self, _feature: PlanFeature, _usage: &PlanUsage) -> Result<(), Error> {
        match self { Self::SelfHosted => Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_hosted_allows_everything_at_any_size() {
        let huge = PlanUsage { organizations_owned: 10_000, members: 1_000_000, guests: 1_000_000, teams: 100_000 };
        for f in [PlanFeature::CreateOrganization, PlanFeature::AddMember, PlanFeature::AddGuest, PlanFeature::CreateTeam] {
            assert!(Plans::SelfHosted.check(f, &huge).is_ok());
        }
        assert!(!Plans::SelfHosted.is_gated());
    }
}
```

In `crates/core/src/error/mod.rs` add, after `EmailUnverified`:

```rust
    /// The organization's plan does not include this, or it is at its limit.
    #[error("{0}")]
    PlanLimit(String),
```

Fix the exhaustive match in `crates/api/src/error/mod.rs` now so the workspace builds: map it to `403` with `{"error": msg, "code": "plan_limit"}` as an early return like `EmailUnverified`, and `CoreError::PlanLimit(m) => (StatusCode::FORBIDDEN, m.clone())` in the match.

- [ ] **Step 5: Platform setting model.** Add `pub organization_creation: OrganizationCreation` to `PlatformSettings` and `pub organization_creation: Option<OrganizationCreation>` to `UpdatePlatformSettings`. `PlatformSettings` is `sqlx::FromRow`, and `crates/core` has no `sqlx::Type` enums (text enums such as `ShareAccess` are parsed in the repos). So drop the `FromRow` derive from `PlatformSettings`, read a private `PlatformRow` (with `organization_creation: String`) in `platform.rs`, and convert with `OrganizationCreation::parse` (`map_err(Error::msg)`). Keep the JSON shape unchanged apart from the new field.

- [ ] **Step 6: Run:** `cargo test -j 2 -p linkdean-core` → PASS. `cargo build -j 2 --workspace` (platform.rs needs Task 2's column, so if it fails only on `organization_creation` in `platform.rs` SQL, finish Task 2 Step 1–2 before committing). fmt, clippy.

- [ ] **Step 7: Commit** (together with Task 2 if Step 6 needed it): "Add organization roles, rules and the plans seam."

---

### Task 2: Migration, organizations, members and the audit log

**Files:**
- Create: `migrations/045_organizations.sql`, `crates/db/src/repo/organizations/{mod,access,audit,organizations,members}.rs`, `crates/db/tests/organizations.rs`, `crates/db/tests/organization_audit.rs`
- Modify: `crates/db/src/repo/mod.rs` (`mod organizations; pub use organizations::*;`), `crates/db/src/repo/platform.rs`

- [ ] **Step 1: Migration** `migrations/045_organizations.sql`:

```sql
-- Organizations and teams, phase 1: organizations, members, teams, invites and the audit log.
-- No content belongs to an organization yet.

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 80),
    -- The team role every Member and Admin gets in the default team.
    default_team_role TEXT NOT NULL DEFAULT 'editor'
        CHECK (default_team_role IN ('editor', 'viewer')),
    created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE organization_members (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'guest')),
    invited_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id)
);
CREATE INDEX organization_members_user_idx ON organization_members (user_id);

CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 80),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Lets child tables prove a team belongs to their organization.
    UNIQUE (id, organization_id)
);
CREATE UNIQUE INDEX teams_one_default ON teams (organization_id) WHERE is_default;
CREATE UNIQUE INDEX teams_name_unique ON teams (organization_id, lower(btrim(name)));

-- A team member is always a member of the team's organization: leaving the organization
-- removes them from its teams.
CREATE TABLE team_members (
    team_id UUID NOT NULL,
    organization_id UUID NOT NULL,
    user_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('maintainer', 'editor', 'viewer')),
    added_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, user_id),
    FOREIGN KEY (team_id, organization_id) REFERENCES teams (id, organization_id) ON DELETE CASCADE,
    FOREIGN KEY (organization_id, user_id)
        REFERENCES organization_members (organization_id, user_id) ON DELETE CASCADE
);
CREATE INDEX team_members_user_idx ON team_members (user_id);
CREATE INDEX team_members_org_user_idx ON team_members (organization_id, user_id);

-- Invites wait for an account whose verified address matches; no claim token.
CREATE TABLE organization_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invitee_email TEXT NOT NULL
        CHECK (invitee_email = lower(btrim(invitee_email)) AND position('@' IN invitee_email) > 1),
    -- Owner only by promotion.
    role TEXT NOT NULL CHECK (role IN ('admin', 'member', 'guest')),
    invited_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    accepted_at TIMESTAMPTZ,
    accepted_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    declined_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    CHECK (num_nonnulls(accepted_at, declined_at, revoked_at) <= 1),
    UNIQUE (id, organization_id)
);
CREATE UNIQUE INDEX organization_invites_open
    ON organization_invites (organization_id, invitee_email)
    WHERE accepted_at IS NULL AND declined_at IS NULL AND revoked_at IS NULL;
CREATE INDEX organization_invites_email_open
    ON organization_invites (invitee_email)
    WHERE accepted_at IS NULL AND declined_at IS NULL AND revoked_at IS NULL;

CREATE TABLE organization_invite_teams (
    invite_id UUID NOT NULL,
    organization_id UUID NOT NULL,
    team_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('maintainer', 'editor', 'viewer')),
    PRIMARY KEY (invite_id, team_id),
    FOREIGN KEY (invite_id, organization_id)
        REFERENCES organization_invites (id, organization_id) ON DELETE CASCADE,
    FOREIGN KEY (team_id, organization_id) REFERENCES teams (id, organization_id) ON DELETE CASCADE
);

-- Append-only. No foreign key on actor or target: `ON DELETE SET NULL` would be an UPDATE, which
-- the trigger refuses. Labels keep who and what it was after they are gone.
CREATE TABLE organization_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    actor_user_id UUID,
    actor_label TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN (
        'organization_created', 'organization_renamed', 'policy_changed',
        'member_invited', 'invite_resent', 'invite_revoked', 'invite_accepted', 'invite_declined',
        'member_role_changed', 'member_removed', 'member_left',
        'team_created', 'team_renamed', 'team_member_added', 'team_member_removed', 'team_role_changed'
    )),
    target_type TEXT NOT NULL CHECK (target_type IN ('organization', 'member', 'invite', 'team')),
    target_id UUID,
    target_label TEXT NOT NULL DEFAULT '',
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX organization_audit_events_org_created
    ON organization_audit_events (organization_id, created_at DESC);
CREATE INDEX organization_audit_events_org_actor
    ON organization_audit_events (organization_id, actor_user_id, created_at DESC);
CREATE INDEX organization_audit_events_org_action
    ON organization_audit_events (organization_id, action, created_at DESC);
CREATE INDEX organization_audit_events_created ON organization_audit_events (created_at);

CREATE FUNCTION organization_audit_events_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'organization_audit_events is append-only';
END
$$;
CREATE TRIGGER organization_audit_events_no_update
    BEFORE UPDATE ON organization_audit_events
    FOR EACH ROW EXECUTE FUNCTION organization_audit_events_append_only();

-- Instance setting: who may create organizations.
ALTER TABLE platform_settings ADD COLUMN organization_creation TEXT NOT NULL DEFAULT 'anyone'
    CHECK (organization_creation IN ('anyone', 'instance_admins'));
```

Deletes from the audit table happen only in `purge_old_audit_events` (by age) and by cascade when an organization row is deleted (Phase 4). Check the migration in `pg-check`: `\d team_members`, and try `UPDATE organization_audit_events SET actor_label = 'x'` → error.

- [ ] **Step 2: Platform repo.** In `platform.rs` add `organization_creation` to the four `SELECT`/`RETURNING` lists, and add:

```rust
pub async fn set_organization_creation(pool: &PgPool, who: OrganizationCreation) -> Result<PlatformSettings>
// UPDATE platform_settings SET organization_creation = $1, updated_at = now() WHERE id = TRUE RETURNING …
```

- [ ] **Step 3: Access helpers** `repo/organizations/access.rs`:

```rust
#[derive(Debug, Clone, Copy)]
pub struct OrgMembership { pub organization_id: Uuid, pub user_id: Uuid, pub role: OrgRole }

/// `None` when the user is not a member.
pub async fn organization_membership(pool: &PgPool, organization_id: Uuid, user_id: Uuid) -> Result<Option<OrgMembership>>

/// `NotFound` for non-members, so the API never confirms an organization exists.
pub async fn require_org_member(pool: &PgPool, organization_id: Uuid, user_id: Uuid) -> Result<OrgMembership>

/// `require_org_member`, then `Forbidden` unless `org_allows(role, action)`.
pub async fn require_org_action(pool: &PgPool, organization_id: Uuid, user_id: Uuid, action: OrgAction) -> Result<OrgMembership>

/// Lock the organization row for a membership change, and return the actor's role read under
/// the lock. `NotFound` when the organization is gone or the actor is not a member.
pub(crate) async fn lock_for_change(conn: &mut PgConnection, organization_id: Uuid, actor_id: Uuid) -> Result<OrgRole>
// SELECT id FROM organizations WHERE id = $1 FOR UPDATE; then SELECT role FROM organization_members …

/// Owners other than `except`, under the lock.
pub(crate) async fn other_owners(conn: &mut PgConnection, organization_id: Uuid, except: Uuid) -> Result<i64>
```

Every membership-changing function in Tasks 2–3 starts its transaction with `lock_for_change`. Lock order is always the organization row first, then member and team rows, which rules out deadlocks.

- [ ] **Step 4: Audit** `repo/organizations/audit.rs`:

```rust
pub(crate) struct NewAuditEvent<'a> {
    pub organization_id: Uuid,
    pub actor_user_id: Uuid,
    pub action: AuditAction,
    pub target_type: AuditTarget,
    pub target_id: Option<Uuid>,
    pub target_label: &'a str,
    pub details: serde_json::Value,
}

/// Write one event in the caller's transaction. The actor label is "Name (email)", or the email
/// when the account has no display name, read from `users` in the same statement.
pub(crate) async fn record_audit(conn: &mut PgConnection, e: NewAuditEvent<'_>) -> Result<()>
// INSERT INTO organization_audit_events (organization_id, actor_user_id, actor_label, action, target_type, target_id, target_label, details)
// SELECT $1, u.id, CASE WHEN nullif(btrim(u.display_name), '') IS NULL THEN u.email ELSE btrim(u.display_name) || ' (' || u.email || ')' END, $3, $4, $5, $6, $7
// FROM users u WHERE u.id = $2
// rows_affected == 0 → Error::msg("audit actor missing")

/// Latest `limit` (1..=100) events, newest first. Needs `OrgAction::ViewAuditLog`.
pub async fn list_audit_events(pool: &PgPool, organization_id: Uuid, viewer_id: Uuid, limit: i64) -> Result<Vec<AuditEventView>>

/// Hourly maintenance: events older than a year.
pub async fn purge_old_audit_events(pool: &PgPool) -> Result<u64>
// DELETE FROM organization_audit_events WHERE created_at < now() - interval '1 year'
```

Target labels: an email address for members and invites, the team name for teams, the organization name for the organization.

- [ ] **Step 5: Organizations** `repo/organizations/organizations.rs`:

```rust
/// Create an organization with the caller as its only owner, and its default team, with the
/// caller in it at the default role. One transaction; audits `organization_created` and
/// `team_created`. The API checks the instance setting and the plan first.
pub async fn create_organization(pool: &PgPool, user_id: Uuid, req: &CreateOrganization) -> Result<OrganizationView>

/// The switcher list, by name.
pub async fn list_my_organizations(pool: &PgPool, user_id: Uuid) -> Result<Vec<OrganizationSummary>>

/// The organization, the viewer's role and the teams the viewer can see (`list_teams`).
pub async fn get_organization(pool: &PgPool, organization_id: Uuid, viewer_id: Uuid) -> Result<OrganizationView>

/// `EditSettings`. Audits `organization_renamed` {from, to} and `policy_changed`
/// {policy: "default_team_role", from, to}; nothing when the value is unchanged.
/// Changing the default role does not change existing members' roles.
pub async fn update_organization(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, patch: &UpdateOrganization) -> Result<Organization>

/// For the Plans seam: owned organizations of `user_id`, and members, guests and teams of
/// `organization_id` when given.
pub async fn plan_usage(pool: &PgPool, organization_id: Option<Uuid>, user_id: Uuid) -> Result<PlanUsage>
```

- [ ] **Step 6: Members** `repo/organizations/members.rs`:

```rust
/// `ViewDirectory` (not Guests). Email only with `ViewMemberEmails`. `suspended_by_instance`
/// from an open `user_bans` row (`lifted_at IS NULL AND (ends_at IS NULL OR ends_at > now())`;
/// match `moderation.rs::active_ban`'s condition exactly). Owners first, then by name.
pub async fn list_members(pool: &PgPool, organization_id: Uuid, viewer_id: Uuid) -> Result<Vec<OrganizationMemberView>>

/// Under the lock: `check_role_change(actor, target, new, other_owners(target))`.
/// To Guest: delete the default team row. From Guest to Member or Admin: insert the default team
/// row at the organization's default role (ON CONFLICT DO NOTHING). Audits `member_role_changed`
/// {from, to}. Same role → Ok, no event. Target not a member → NotFound.
pub async fn change_member_role(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, target_user_id: Uuid, role: OrgRole) -> Result<()>

/// Under the lock: actor == target → BadRequest("use leave"); `check_removal`; delete the
/// membership (team rows go by cascade). Audits `member_removed`.
pub async fn remove_member(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, target_user_id: Uuid) -> Result<()>

/// Under the lock: `check_leave(role, other_owners)`; delete; audit `member_left`.
pub async fn leave_organization(pool: &PgPool, organization_id: Uuid, user_id: Uuid) -> Result<()>
```

- [ ] **Step 7: Tests** `crates/db/tests/organizations.rs`. Helpers: `pool_or_skip`, `address(prefix)` → `{prefix}-{uuid}@org.test`, `verified(pool, prefix) -> User` (`create_user` + `mark_email_verified`), and `org_with(pool, owner, [(user, role)])`, which creates an organization and inserts members through the invite path once Task 3 exists. Until then, insert them with raw SQL in a test-only helper, `add_member_sql(pool, org, user, role)`, which also adds the default team row unless the role is Guest. Cases:
  - `create_organization` makes the caller Owner, one default team "General" with `is_default`, the caller listed in it as Editor, and two audit events;
  - an empty or 81-character name → `BadRequest`;
  - `list_my_organizations` returns only the user's organizations;
  - a non-member gets `NotFound` from `get_organization`, `list_members`, `update_organization`;
  - a Guest gets `Forbidden` from `list_members`; a Member sees names and roles with `email: None`; an Admin sees emails;
  - Admin promotes Member → Admin: `Forbidden`; Owner does it: Ok, audited with {from, to};
  - the last Owner demoting themselves → `Conflict`; with a second Owner → Ok;
  - **race:** two Owners each demote the other concurrently (`tokio::join!` on two `change_member_role` calls) → exactly one succeeds and one Owner remains (assert by counting owners);
  - Member → Guest removes the default team row; Guest → Member restores it at the default role;
  - `remove_member` by Admin of a Member: membership and every `team_members` row in that organization gone, other organizations untouched; by Admin of an Admin: `Forbidden`;
  - `leave_organization` by the last Owner → `Conflict`; by a Member → Ok and audited `member_left`;
  - `update_organization { default_team_role: Some(Maintainer) }` → `BadRequest`; `Viewer` → Ok, `policy_changed` audited; existing members keep their role;
  - an instance ban (`db::ban_user`, check its signature) shows `suspended_by_instance: true` and keeps the membership;
  - `plan_usage` counts members, guests and teams.

`crates/db/tests/organization_audit.rs`:
  - an `UPDATE` on `organization_audit_events` fails (raw SQL; assert the error text contains "append-only");
  - `purge_old_audit_events` deletes an event whose `created_at` is 366 days old and keeps one at 364 days. Insert the old rows directly with raw SQL; the trigger blocks only `UPDATE`;
  - `list_audit_events` needs `ViewAuditLog` (Member → `Forbidden`), returns newest first, caps `limit` at 100, and keeps the actor label after the actor leaves.

- [ ] **Step 8:** Full `cargo test -j 2 --workspace` green (existing suites untouched), fmt, clippy. **Commit** "Store organizations, members and an append-only audit log."

---

### Task 3: Teams and invites

**Files:**
- Create: `crates/db/src/repo/organizations/{teams,invites}.rs`, `crates/db/tests/organization_teams.rs`, `crates/db/tests/organization_invites.rs`
- Modify: `crates/db/src/repo/organizations/mod.rs`; in `crates/db/tests/organizations.rs`, replace `add_member_sql` with the invite path

- [ ] **Step 1: Teams** `teams.rs`:

```rust
#[derive(Debug, Clone)]
pub struct TeamAccess { pub team_id: Uuid, pub organization_id: Uuid, pub is_default: bool, pub org_role: OrgRole, pub listed: Option<TeamRole>, pub effective: Option<TeamRole> }

/// `NotFound` for non-members of the organization, and for Guests not listed in the team.
/// Members not listed get `effective: None` (they can see the name and join).
pub async fn team_access(pool: &PgPool, team_id: Uuid, user_id: Uuid) -> Result<TeamAccess>

/// `team_access`, then `Forbidden` unless `effective >= min`.
pub async fn require_team_role(pool: &PgPool, team_id: Uuid, user_id: Uuid, min: TeamRole) -> Result<TeamAccess>

/// Default team first, then by name. Guests see only teams they are listed in.
pub async fn list_teams(pool: &PgPool, organization_id: Uuid, viewer_id: Uuid) -> Result<Vec<TeamView>>

/// `ManageTeams` (Owners and Admins in Phase 1; the "who creates teams" policy is Phase 5).
/// The creator is not listed (Owners and Admins are Maintainer anyway). A duplicate name
/// (case-insensitive) → Conflict("a team with this name already exists"). Audits `team_created`.
pub async fn create_team(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, name: &str) -> Result<TeamView>

/// Maintainer. Audits `team_renamed` {from, to}.
pub async fn rename_team(pool: &PgPool, team_id: Uuid, actor_id: Uuid, name: &str) -> Result<TeamView>

/// Viewer and above (listed or effective). Guests in the team see its members: this is not the directory.
pub async fn list_team_members(pool: &PgPool, team_id: Uuid, viewer_id: Uuid) -> Result<Vec<TeamMemberView>>

/// Maintainer. The user must be a member of the organization: the composite foreign key refuses
/// anyone else; map that violation (SQLSTATE 23503) to BadRequest("not a member of this organization").
/// Already listed → Conflict. Audits `team_member_added` {role}.
pub async fn add_team_member(pool: &PgPool, team_id: Uuid, actor_id: Uuid, user_id: Uuid, role: TeamRole) -> Result<()>

/// Maintainer. Audits `team_role_changed` {from, to}.
pub async fn change_team_role(pool: &PgPool, team_id: Uuid, actor_id: Uuid, user_id: Uuid, role: TeamRole) -> Result<()>

/// Maintainer, or the user themselves (leaving a team). In the default team, only Guests can be
/// removed: anyone else → BadRequest("everyone stays in the default team; change their role instead").
/// Audits `team_member_removed` {left: actor == user}.
pub async fn remove_team_member(pool: &PgPool, team_id: Uuid, actor_id: Uuid, user_id: Uuid) -> Result<()>

/// Members, Admins and Owners join any team at the organization's default team role; Guests →
/// Forbidden; already listed → Conflict. Audits `team_member_added` {role, joined: true}.
pub async fn join_team(pool: &PgPool, team_id: Uuid, user_id: Uuid) -> Result<TeamView>
```

All writes take `lock_for_change` on the team's organization first.

- [ ] **Step 2: Invites** `invites.rs`. Reuse email normalization: `pending_shares.rs` and `email_verification.rs` each have an identical private `normalize_email`. Move one copy to `crates/db/src/repo/users.rs` as `pub(crate) fn normalize_email`, delete both private copies, and use it from all three files. Do not add a third copy.

```rust
/// `check_invite_role`. Refuses an address that already belongs to a member (Conflict
/// "already a member of this organization"). Teams must be in this organization (the composite
/// foreign key; map 23503 to BadRequest("unknown team")). Upserts the open invite for
/// (organization, address): a repeat replaces role, teams, inviter and `expires_at = now() + ttl`.
/// Audits `member_invited` {email, role, teams, renewed}. Returns the view and whether a
/// verified account holds the address now.
pub async fn create_organization_invite(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, req: &CreateOrganizationInvite, ttl: chrono::Duration) -> Result<(OrganizationInviteView, bool)>

/// `InviteMembers`. Open, unexpired invites, newest first. Expired ones are not listed.
pub async fn list_organization_invites(pool: &PgPool, organization_id: Uuid, viewer_id: Uuid) -> Result<Vec<OrganizationInviteView>>

/// `InviteMembers`, and `GrantAdmin` for an Admin invite. Audits `invite_revoked`.
pub async fn revoke_organization_invite(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, invite_id: Uuid) -> Result<()>

/// `InviteMembers` (and `GrantAdmin` for an Admin invite). Renews `expires_at`, works on an expired
/// invite that was never answered. Audits `invite_resent`. Returns the view and the verified flag.
pub async fn resend_organization_invite(pool: &PgPool, organization_id: Uuid, actor_id: Uuid, invite_id: Uuid, ttl: chrono::Duration) -> Result<(OrganizationInviteView, bool)>

/// Open, unexpired invites to the account's current address, when it is verified; otherwise an
/// empty list and the count in `waiting_for_verification`. Leaves out organizations the user is
/// already in.
pub async fn list_my_organization_invites(pool: &PgPool, user_id: Uuid) -> Result<MyOrganizationInvites>

/// In one transaction: the invite row `FOR UPDATE`; it must be open, unexpired and addressed to the
/// account's current address (else NotFound); an unverified account → BadRequest("verify your email
/// address to accept this invite"). Never use `Error::EmailUnverified` here: the web app treats it
/// as the hard gate. Then `lock_for_change`-style lock on the organization (the invitee is not a
/// member yet, so lock the organization row directly), insert the membership (already a member →
/// mark accepted, no change), add the default team at the default role unless Guest (an invite
/// team entry for the default team overrides the role), add the invite's teams, mark accepted,
/// audit `invite_accepted` with the invitee as actor.
pub async fn accept_organization_invite(pool: &PgPool, user_id: Uuid, invite_id: Uuid) -> Result<OrganizationSummary>

/// Same checks; marks declined; audits `invite_declined`.
pub async fn decline_organization_invite(pool: &PgPool, user_id: Uuid, invite_id: Uuid) -> Result<()>
```

`lock_for_change` requires a member actor, so add `pub(crate) async fn lock_organization(conn, organization_id) -> Result<()>` in `access.rs` and call it from `lock_for_change` and here.

- [ ] **Step 3: Tests** `crates/db/tests/organization_teams.rs`:
  - Admin creates a team; Member → `Forbidden`; Guest → `Forbidden`; duplicate name with other case → `Conflict`;
  - an Owner sees every team with `effective_role: maintainer` without being listed;
  - a Member sees all team names, `effective_role: None` for teams they are not in, and can `join_team` (gets the default role, audited `joined: true`);
  - a Guest lists only the teams they are listed in and `team_access` to another team is `NotFound`; `join_team` → `Forbidden`;
  - a Maintainer (listed Member) adds a member, changes a role, removes a member; an Editor doing any of these → `Forbidden`;
  - adding a user from another organization, or no organization → `BadRequest`;
  - removing a Member from the default team → `BadRequest`; removing a Guest from it (after an Admin added them) → Ok;
  - leaving a non-default team yourself as an Editor → Ok;
  - `rename_team` by Maintainer → Ok and audited; by Viewer → `Forbidden`;
  - `remove_member` of the organization removes the person from every team (cascade).

`crates/db/tests/organization_invites.rs`:
  - Admin invites `x@…` as Member with team T (Viewer); `list_organization_invites` shows it; `can_accept_now` is false while no account exists;
  - a repeat invite updates role and teams and keeps one open row, audited `renewed: true`;
  - Admin inviting as Admin → `Forbidden`; anyone inviting as Owner → `BadRequest`; Member inviting → `Forbidden`;
  - inviting an existing member's address → `Conflict`; a team from another organization → `BadRequest`;
  - an account registered with the address but unverified: `list_my_organization_invites` returns no invites and `waiting_for_verification: 1`; `accept` → `BadRequest`; after `mark_email_verified`, the invite is listed and `accept` makes them Member, in the default team at the default role and in T as Viewer, audited with the invitee as actor;
  - a Guest invite: accepting adds no default team row;
  - the address matching is case-insensitive (`X@Org.Test` invite, `x@org.test` account);
  - an expired invite (set `expires_at` in the past with raw SQL) is not listed and `accept` → `NotFound`; `resend` renews it;
  - another account (different address) accepting someone else's invite id → `NotFound`;
  - `decline` → not listed again; audited;
  - `revoke` → invitee no longer sees it; audited;
  - after an email change to a new verified address (`db::admin_change_email`), invites to the new address are listed and those to the old one are not;
  - accepting twice (second call) → `NotFound` (no longer open).

- [ ] **Step 4:** Full suite, fmt, clippy. **Commit** "Add teams, team roles and organization invites."

---

### Task 4: Invite email template

**Files:**
- Create: `crates/mail/templates/organization_invite.mjml`, `.subject`, `.sample.json`
- Modify: `crates/mail/src/render.rs` (`template!("organization_invite"),` in `TEMPLATES`)

| Variables | Subject | Content |
| --- | --- | --- |
| `inviter`, `organization`, `role` ("an admin", "a member", "a guest"), `url`, `has_account` (bool), `expires` (e.g. "December 3, 2026") | `{{ inviter }} invited you to {{ organization }}` | Heading "Join {{ organization }}". "{{ inviter }} invited you to join {{ organization }} on {{ site.instance }} as {{ role }}." With `has_account`: "Open your invites to accept." Button "See the invite" → `url`. Without: "Create an account with this email address and verify it to accept." Button "Create an account" → `url`. Footer line: "The invite expires on {{ expires }}. If you didn't expect it, ignore this email." |

Copy the structure of `share_invite_pending.mjml` (layout and macros). Sample data: `"inviter": "Ada Lovelace"`, `"organization": "Acme Inc."`, `"role": "a member"`, `"url": "https://links.example.com/invites"`, `"has_account": true`, `"expires": "December 3, 2026"`.

- [ ] Tests in `render.rs`'s test module, following the existing ones: the sample renders, the text part holds the whole URL on one line, and an organization named `<b>Acme</b>` renders as text in the HTML part. `cargo test -j 2 -p linkdean-mail` green.
- [ ] **Commit** "Add the organization invite email."

---

### Task 5: API

**Files:**
- Create: `crates/api/src/plans.rs`, `crates/api/src/organization_mail.rs`, `crates/api/src/routes/organizations/{mod,members,teams,invites}.rs`
- Modify: `crates/api/src/main.rs` (`mod plans; mod organization_mail;`), `state/mod.rs`, `routes/mod.rs`, `maintenance.rs`, `routes/admin/mod.rs`

- [ ] **Step 1: Plans seam wiring.** `AppState` gets `pub plans: linkdean_core::plans::Plans`, set to `Plans::SelfHosted` in `AppState::new` (grep for any other `AppState {` literal and set it there too). `crates/api/src/plans.rs`:

```rust
//! The only caller of `Plans::check`. Self-hosted is never gated, so it runs no queries.

use crate::error::ApiResult;
use crate::state::AppState;
use linkdean_core::plans::PlanFeature;
use linkdean_db::{self as db};
use uuid::Uuid;

pub async fn require(state: &AppState, organization_id: Option<Uuid>, user_id: Uuid, feature: PlanFeature) -> ApiResult<()> {
    if !state.plans.is_gated() {
        return Ok(());
    }
    let usage = db::plan_usage(&state.pool, organization_id, user_id).await?;
    state.plans.check(feature, &usage)?;
    Ok(())
}
```

Call it in exactly three places: create organization (`CreateOrganization`, `None`), accept invite (`AddMember` or `AddGuest` by the invite's role, with the invite's organization: read it from `list_my_organization_invites` or add a small `db::organization_invite_target(pool, user_id, invite_id) -> Result<(Uuid, OrgRole)>`), create team (`CreateTeam`). Check with `grep -rn "plans::require" crates/api/src` that there are three.

- [ ] **Step 2: Invite email** `organization_mail.rs`:

```rust
/// Email an invite just created or resent. Ignores the invitee's notification preferences (it is
/// an invitation, not a notification) but counts on the inviter's hourly invite budget
/// (`invite_email_limit`, `AppState::invite_limiter`), shared with share invites. Never fails the
/// request: errors are logged and reported as not sent.
pub async fn send_organization_invite(state: &AppState, inviter: &User, organization_name: &str, invite: &OrganizationInviteView, has_account: bool) -> InviteEmail
```

Body: return `InviteEmail::default()` with mail off (`state.mail.is_none()`); check `invite_email_limit(inviter.id)` against `state.invite_limiter` → over → `InviteEmail { sent: false, limited: true }`; build data (`inviter: owner_label(inviter)` via `crate::notification_mail::owner_label`, flattened with its `one_line`; make `one_line` `pub(crate)`). `url` is `{app_url}/invites` when `has_account`, else `{app_url}/register`. `expires` is formatted `%B %-d, %Y`, and `role` is the article phrase. Then `crate::mail::enqueue(state, &invite.invitee_email, "organization_invite", &data)`, and on `Queued::Queued` record the limit. Reuse `InviteEmail` and `with_invite_email` from `notification_mail.rs`.

- [ ] **Step 3: Routes** under `crates/api/src/routes/organizations/`, merged in `routes/mod.rs`. All use `AuthUser`. Paths, as the nested router sees them:

| Method and path | Handler → db | Response |
| --- | --- | --- |
| `GET /organizations` | `list_my_organizations` | `{organizations}` |
| `POST /organizations` `{name}` | instance setting check (below), `plans::require`, `create_organization` | `{organization: OrganizationView}` |
| `GET /organizations/{id}` | `get_organization` | `{organization: OrganizationView}` |
| `PATCH /organizations/{id}` `{name?, default_team_role?}` | `update_organization` | `{organization}` |
| `GET /organizations/{id}/members` | `list_members` | `{members}` |
| `PATCH /organizations/{id}/members/{user_id}` `{role}` | `change_member_role` | `{ok: true}` |
| `DELETE /organizations/{id}/members/{user_id}` | `remove_member` | `{ok: true}` |
| `POST /organizations/{id}/leave` | `leave_organization` | `{ok: true}` |
| `GET /organizations/{id}/teams` | `list_teams` | `{teams}` |
| `POST /organizations/{id}/teams` `{name}` | `plans::require`, `create_team` | `{team}` |
| `PATCH /teams/{id}` `{name}` | `rename_team` | `{team}` |
| `POST /teams/{id}/join` | `join_team` | `{team}` |
| `GET /teams/{id}/members` | `list_team_members` | `{members}` |
| `POST /teams/{id}/members` `{user_id, role}` | `add_team_member` | `{ok: true}` |
| `PATCH /teams/{id}/members/{user_id}` `{role}` | `change_team_role` | `{ok: true}` |
| `DELETE /teams/{id}/members/{user_id}` | `remove_team_member` | `{ok: true}` |
| `GET /organizations/{id}/invites` | `list_organization_invites` | `{invites}` |
| `POST /organizations/{id}/invites` `{email, role, teams}` | `create_organization_invite(ttl = INVITE_DAYS)`, then `send_organization_invite` | `with_invite_email(&json!({"invite": view}), email)` → `{invite, email_sent, email_limited?}` |
| `POST /organizations/{id}/invites/{invite_id}/resend` | `resend_organization_invite`, `send_organization_invite` | same shape |
| `DELETE /organizations/{id}/invites/{invite_id}` | `revoke_organization_invite` | `{ok: true}` |
| `GET /organization-invites` | `list_my_organization_invites` | `MyOrganizationInvites` |
| `POST /organization-invites/{id}/accept` | `plans::require`, `accept_organization_invite` | `{organization: OrganizationSummary}` |
| `POST /organization-invites/{id}/decline` | `decline_organization_invite` | `{ok: true}` |
| `GET /organizations/{id}/audit-log` | `list_audit_events(limit 100)` | `{events}` |

Create rule: when `platform.organization_creation == InstanceAdmins && !user.is_admin`, return `Error::BadRequest("only instance admins can create organizations here".into())` (400, so the web app can show the message). Leave the hard gate unchanged: these paths are not under `/auth/`, so unverified accounts are gated when the gate is on. That is fine.

Tell the route modules to keep handlers thin (validate → db → map), per `crates/AGENTS.md`.

- [ ] **Step 4: Instance setting.** In `patch_platform_settings` add `if let Some(who) = req.organization_creation { settings = db::set_organization_creation(&state.pool, who).await?; }`. `GET /auth/bootstrap-status` (`bootstrap_status` in `routes/auth/mod.rs`, which the web app reads through `useBootstrap`) adds `"organization_creation": platform.organization_creation.as_str()`, so the web app can hide "Create an organization" from non-admins when it is `instance_admins`.

- [ ] **Step 5: Maintenance.** In `maintenance.rs::run_once` add a `purge_old_audit_events` block, matching the existing `purge_old_account_tokens` block (log count at `info` when non-zero, `warn` on error).

- [ ] **Step 6: Smoke.** Use the throwaway stack from the email plans: API on `127.0.0.1:18081` against the `smoke` database, Mailpit `ld-mailpit`, script `/tmp/claude-502/ldreset/run.sh` if it still exists, otherwise rebuild it per the `throwaway-postgres-sql-check` skill. Test users `uiqa-org-*`; every POST, PATCH and DELETE carries `X-Linkdean-Client: web`.
  - Owner A creates "Acme Inc." → `GET /organizations` lists it with role owner; the default team "General" is listed;
  - A invites `uiqa-org-b@…` (no account) as Member with a new team "Research" (Viewer) → `email_sent: true`, and Mailpit shows `organization_invite` with "Create an account" → `/register`;
  - B registers with that address → `GET /organization-invites` → `waiting_for_verification: 1`; B verifies through the Mailpit link → the invite is listed; accept → B is a Member in General (Editor) and Research (Viewer);
  - A invites existing verified C → the email says "See the invite" → `/invites`; C declines → not listed;
  - B (Member) `GET /organizations/{id}/members` → names, no emails; `POST /organizations/{id}/invites` → 403;
  - A tries to leave → 409; A promotes B to Owner, then A leaves → 200;
  - audit log (as B, now Owner) lists every step above, newest first;
  - Admin sets `organization_creation: instance_admins` → a non-admin `POST /organizations` → 400; reset to `anyone`;
  - a non-member `GET /organizations/{id}` → 404.
  Clean up: delete the smoke organizations (`DELETE FROM organizations WHERE name LIKE 'uiqa-%'`; name them that way), users and outbox rows; stop the API and Mailpit.
- [ ] Full suite, fmt, clippy. **Commit** "Add organization, team and invite routes."

---

### Task 6: Web data layer

**Files:**
- Create: `apps/web/src/schemas/organizations/{organizations.ts,index.ts}`, `apps/web/src/hooks/organizations/{useOrganizations.ts,useOrganizationMembers.ts,useTeams.ts,useOrganizationInvites.ts,useMyOrganizationInvites.ts,useAuditLog.ts,useCurrentWorkspace.ts,index.ts}`, `apps/web/src/services/workspace/{workspace.ts,workspace.test.ts,index.ts}`, `apps/web/src/hooks/organizations/useMyOrganizationInvites.test.tsx`
- Modify: `apps/web/src/schemas/index.ts`, `apps/web/src/hooks/index.ts`, `apps/web/src/services/index.ts`, `apps/web/src/api/api.ts`, `apps/web/src/schemas/auth/auth.ts` (`organization_creation: z.enum(['anyone', 'instance_admins']).default('anyone')` on `bootstrapStatusSchema` and `platformSettingsSchema`)

- [ ] **Step 1: Schemas** (zod, matching Task 1's views field for field): `orgRoleSchema = z.enum(['owner','admin','member','guest'])`, `teamRoleSchema = z.enum(['maintainer','editor','viewer'])`, `organizationSummarySchema`, `organizationSchema`, `teamViewSchema`, `organizationViewSchema`, `organizationMemberSchema` (`email: z.string().nullish()`), `teamMemberSchema`, `organizationInviteSchema`, `myOrganizationInvitesSchema`, `auditEventSchema`, and envelope schemas for each response in Task 5's table.

- [ ] **Step 2: API functions** in `api/api.ts`, one per route in Task 5 (`listOrganizations`, `createOrganization`, `getOrganization`, `updateOrganization`, `listOrganizationMembers`, `changeOrganizationMemberRole`, `removeOrganizationMember`, `leaveOrganization`, `listTeams`, `createTeam`, `renameTeam`, `joinTeam`, `listTeamMembers`, `addTeamMember`, `changeTeamRole`, `removeTeamMember`, `listOrganizationInvites`, `inviteToOrganization`, `resendOrganizationInvite`, `revokeOrganizationInvite`, `myOrganizationInvites`, `acceptOrganizationInvite`, `declineOrganizationInvite`, `organizationAuditLog`), following the existing `req(path, init, schema)` calls.

- [ ] **Step 3: Workspace from the URL** (pure, in `services/workspace/workspace.ts`):

```ts
export type Workspace = { kind: 'personal' } | { kind: 'organization'; organizationId: string }

/** `/orgs/{id}` and everything under it is that organization; anything else is Personal. */
export function workspaceFromPath(pathname: string): Workspace {
  const m = /^\/orgs\/([^/]+)/.exec(pathname)
  return m ? { kind: 'organization', organizationId: decodeURIComponent(m[1]) } : { kind: 'personal' }
}
```

Test: `/library` → personal; `/orgs/abc` and `/orgs/abc/settings` → organization `abc`; `/orgsx` → personal.

`useCurrentWorkspace()` wraps it with `useRouterState({ select: (s) => s.location.pathname })` and returns `{ workspace, organization }`, where `organization` is the matching `OrganizationSummary` from `useOrganizations()`, or `undefined`.

- [ ] **Step 4: Hooks**, one file per concern, query keys `['organizations']`, `['organization', id]`, `['organization-members', id]`, `['team-members', teamId]`, `['organization-invites', id]`, `['my-organization-invites']`, `['organization-audit', id]`. Every mutation invalidates the keys it changes plus `['organization-audit', id]`. `useMyOrganizationInvites(enabled)` polls every 30 s like `useShareInvites`. Accept invalidates `['organizations']` and `['my-organization-invites']`.

- [ ] **Step 5: Tests.** `workspace.test.ts` as above. In `useMyOrganizationInvites.test.tsx` (mock `@/api` like `NotificationEmailsPanel.test.tsx`), accepting refetches organizations and invites.
- [ ] All web checks green. **Commit** "Add the organizations data layer to the web app."

---

### Task 7: Web shell (switcher, organization sidebar, create, invites)

**Files:**
- Create: `apps/web/src/pages/app-shell/WorkspaceSwitcher.tsx` (+ `.test.tsx`), `apps/web/src/pages/app-shell/OrganizationSidebar.tsx` (+ `.test.tsx`), `apps/web/src/components/organization/{CreateOrganizationModal.tsx,index.ts}` (+ test), `apps/web/src/pages/organization-invites/{OrganizationInvitesPage.tsx,index.ts}` (+ test), `apps/web/src/pages/organization/{OrganizationHomePage.tsx,TeamSpacePage.tsx,index.ts}`, `apps/web/src/routes/_app/invites.tsx`, `apps/web/src/routes/_app/orgs.$orgId.index.tsx`, `apps/web/src/routes/_app/orgs.$orgId.teams.$teamId.tsx`
- Modify: `apps/web/src/pages/app-shell/AppSidebar.tsx`, `AppShellHeader.tsx` (+ its test), `AppShell.tsx`, `AppShell.module.css`, `apps/web/src/components/index.ts`, `apps/web/src/pages/index.ts`, `apps/web/src/routeTree.gen.ts`

Load skill `linkdean-web-navigation` first. Each action keeps one control per width:

| Action | Desktop | Phone |
| --- | --- | --- |
| Switch workspace | `WorkspaceSwitcher` at the top of the sidebar, shown only with at least one organization | Same, at the top of the burger sidebar |
| Create an organization | Account menu item (hidden when `organization_creation` is `instance_admins` and the user is not an instance admin) | Same account menu |
| Organization invites | Account menu item "Organization invites" with a count badge, shown when the count or `waiting_for_verification` is above 0 | Same |
| Organization settings | Organization sidebar footer, Owners and Admins only, replacing "Config options" there | Same |

- [ ] **Step 1: `WorkspaceSwitcher`.** A Mantine `Menu` whose target shows the current workspace: "Personal" with the user's avatar, or the organization name with an initials `Avatar` (color from a hash of the id; the logo is Phase 5). Items: "Personal" (→ `/`), then each organization (→ `/orgs/{id}`), each with a check on the current one. Test: hidden with no organizations; lists Personal and each organization; the current one is marked; choosing navigates.

- [ ] **Step 2: `OrganizationSidebar`.** Rendered by `AppSidebar` instead of the Personal sections when `useCurrentWorkspace().workspace.kind === 'organization'`. It shows a "Teams" group listing the teams with `my_role` or `effective_role` set (default team first, as the API orders them; Owners and Admins see every team) as `NavLink`s to `/orgs/{id}/teams/{teamId}`. It has an "Organization" link to `/orgs/{id}`, and a footer with "Organization settings" (`/orgs/{id}/settings`, Owners and Admins). Personal's Library, Quick views, folders and Library health do not show inside an organization. Test: an Owner sees settings; a Member does not; teams in API order; a Guest sees only their teams.

- [ ] **Step 3: Header.** In `AppShellHeader.tsx`'s account menu, add "Create an organization" (opens `CreateOrganizationModal`) and "Organization invites" (→ `/invites`, `Badge` with the count) after "Config options". In `AppShell.tsx`, give `<MantineAppShell.Header px={0}>` a new `className={classes.header}` and `data-workspace="organization"` inside an organization (`AppShell.module.css` has no header class yet), and add the tint:

```css
.header[data-workspace='organization'] {
  background: color-mix(in srgb, var(--mantine-color-violet-light) 35%, var(--mantine-color-body));
}
```
 Check contrast in both themes in the browser step. Test: the menu shows both items for a solo user (invites only with a count above 0); "Create an organization" is hidden for a non-admin when the setting is `instance_admins`.

- [ ] **Step 4: `CreateOrganizationModal`.** A name field (1–80 characters, trimmed) and the text "You'll be its owner. Your personal library stays private." with a "Create organization" button. On success it closes and navigates to `/orgs/{id}`; errors show inline. Test: the button is disabled while the name is empty; submitting calls the API with the trimmed name and navigates.

- [ ] **Step 5: Organization invites page** (`/invites`, signed in). A list of invites ("{inviter} invited you to {organization} as {role}", expires date) with Accept and Decline. After accepting, show "You joined {organization}" and a button "Open {organization}" → `/orgs/{id}`. With `waiting_for_verification > 0`: an alert "Verify your email address to see {n} organization invites." with the existing Resend action (`hooks/auth/useEmailVerification.ts`). Empty state: "No organization invites." Loading, error with retry, and success states (apps/web AGENTS.md §5). Test: accept and decline call the API; the verification alert shows with a count.

- [ ] **Step 6: Organization home and team space pages.** `OrganizationHomePage` (`/orgs/$orgId`) shows the organization name and a list of the viewer's teams (cards with member count and the viewer's role). Members also see other teams with a "Join" button. A "Leave organization" action is in a menu, behind `ConfirmModal`; a `409` shows "Make someone else an owner before you leave." `TeamSpacePage` (`/orgs/$orgId/teams/$teamId`) shows the team name, the empty state "This team space is empty." (`EmptyState`), and the team members (`useTeamMembers`). Maintainers get add (pick from organization members), change role and remove controls; non-default teams get "Leave team"; Maintainers can rename the team. A `404` from the API on either page shows "This organization isn't available." with a link to Personal.

- [ ] **Step 7: Routes.** `routes/_app/invites.tsx`, `routes/_app/orgs.$orgId.index.tsx` and `routes/_app/orgs.$orgId.teams.$teamId.tsx` render those pages. Run `npx tsr generate`. No `isPublicPath` change is needed: all of them are signed-in pages.

- [ ] All web checks green. **Commit** "Add the workspace switcher and organization pages to the web app."

---

### Task 8: Web organization settings and instance setting

**Files:**
- Create: `apps/web/src/pages/organization/{OrganizationSettingsPage.tsx,MembersPanel.tsx,InviteMemberModal.tsx,PendingInvitesList.tsx,TeamsPanel.tsx,PoliciesPanel.tsx,AuditLogPanel.tsx}` (+ tests for `MembersPanel`, `InviteMemberModal`, `PoliciesPanel`, `AuditLogPanel`), `apps/web/src/routes/_app/orgs.$orgId.settings.tsx`
- Modify: `apps/web/src/pages/settings/AdminPlatformSettings.tsx` (+ its test), `apps/web/src/routeTree.gen.ts`

- [ ] **Step 1: Settings page** `/orgs/$orgId/settings?tab=members|teams|policies|audit`, with the same vertical-tabs-on-wide and scrolling-row-on-phone layout as `SettingsPage.tsx`; copy its `useMediaQuery('(min-width: 62em)', true)` pattern. Owners and Admins only: anyone else is redirected to `/orgs/$orgId`. The Danger zone tab arrives in Phase 4, so do not render it now.

- [ ] **Step 2: Members tab.** A table of name, email, role and joined date, with a "Suspended by instance" badge. Owners and Admins get a role `Select` per row, offering only the roles `check_role_change` allows (Owners: all four; Admins: Member and Guest, and only on Member and Guest rows), and "Remove" behind `ConfirmModal`. `409` and `403` show the API message. "Invite people" opens `InviteMemberModal`: email, role (`Select`; "Admin" only for Owners), teams (`MultiSelect` with a role per chosen team, default Editor). After success it shows "We emailed them an invite.", or the share dialog's limit warning when `email_limited`, or "Email is off. Tell them to sign in with this address to accept." when `email_sent` is false and mail is off. `PendingInvitesList` below the table lists open invites with Resend and Revoke, plus a "No account yet" badge when `can_accept_now` is false. Tests: an Admin sees no "Admin" role option; the invite modal sends teams with roles; Revoke calls the API.

- [ ] **Step 3: Teams tab.** Teams with member counts, a "Create team" button (name modal) and rename. Each team links to its team space page for membership.

- [ ] **Step 4: Policies tab.** "Default team role" `SegmentedControl` (Editor / Viewer) with the description "The role new members get in {default team name}. Existing members keep theirs." It saves on change. Test: changing it calls `updateOrganization({ default_team_role: 'viewer' })`. The organization name field also lives here (rename).

- [ ] **Step 5: Audit log tab.** A plain table of when (local time), who, what (a sentence per action, built in a pure `services/organization/auditSentence.ts` with a unit test per `AuditAction`, e.g. `member_role_changed` → "changed Bob's role from Member to Admin"), and target. Caption: "Latest 100 events. Filters and export are coming." Test: rows render the sentence; empty state.

- [ ] **Step 6: Instance setting.** In `AdminPlatformSettings.tsx`, add a `Select` "Who can create organizations" with the options "Anyone" and "Instance admins only", saving through `updatePlatformSettings({ organization_creation })`. Test: changing it sends the field.

- [ ] **Step 7:** `npx tsr generate`; all web checks green. **Commit** "Add organization settings to the web app."

---

### Task 9: Docs, browser check, close-out

- [ ] **Docs** (skill `linkdean-self-host-docs`; take every value from the code):
  - ARCHITECTURE: a new "Organizations" section covering the tables, the composite keys, the role rules (`crates/core/src/models/organizations.rs`), the organization row lock, invite matching by verified address with a 30-day expiry, the append-only audit log (trigger, no foreign keys on actor or target, hourly purge after a year), the Plans seam (`crates/core/src/plans.rs`, `crate::plans::require`, self-hosted never gated) and the `/orgs/{id}` workspace URLs. Add `Organization`, `Team`, `OrganizationInvite` and `AuditEvent` to "Domain (v1)". Add the audit purge to the hourly maintenance list.
  - FEATURES: an "Organizations" section listing what Phase 1 ships.
  - SELF_HOSTED: the "Who can create organizations" instance setting; that organization invites need outbound email or admin-verified accounts; the spec's statement that a self-hosted operator can read organization data from the database ("no in-app access", not "technically impossible").
  - ROADMAP item 12: keep the item, and add "Phase 1 (organizations, members, teams, invites, audit log recording) shipped; next: team spaces" with a link to the phases doc.
- [ ] **Browser check** (skill `browser-ui-verification`, throwaway stack as in Task 5, desktop and phone widths):
  1. A solo user sees no switcher and "Create an organization" in the account menu → create "uiqa-Acme" → lands on the organization home, the switcher appears, the header is tinted, and the sidebar lists "General".
  2. Invite a new address as Member with a new team → Mailpit email → register → the verify prompt on `/invites` shows 1 waiting → verify → accept → the switcher lists the organization and the sidebar shows both teams.
  3. As the Member: no "Organization settings"; the members list in a team space shows no emails; "Join" on another team works.
  4. As the Owner: change roles (Admins can't offer Admin), remove a member, revoke and resend an invite, change the default team role, and see each in the Audit log.
  5. Leave as the last Owner → the explanatory message; promote someone else → leave works.
  6. Switch back to Personal → the library is unchanged; the Personal sidebar is unchanged.
  7. Instance admin sets "Instance admins only" → a non-admin no longer sees "Create an organization".
  8. No unexplained 4xx, no console errors; screenshots of the switcher, organization sidebar, invites page, members tab and audit log at both widths, looked at.
  Clean up the test organizations, users and outbox rows; stop the servers and Mailpit.
- [ ] **Commit** "Document organizations phase 1." Run `graphify update .` and report its result.

---

## Self-review against the spec (Phase 1 scope)

| Spec item | Task |
| --- | --- |
| Vocabulary | All tasks; Repo rules |
| A user in several organizations or none; Personal unchanged; solo users see only "Create an organization" | 2 (`list_my_organizations`), 7 (switcher hidden, account menu) |
| Every organization has a default team that cannot be deleted | 2 (`teams_one_default`; no delete route exists in Phase 1) |
| Organization roles and the last-owner rule | 1 (rules), 2 (lock, race test) |
| Team roles; Owners and Admins act as Maintainer; members join the default team as Editor, or Viewer by policy; Guests never auto-join | 1 (`effective_team_role`), 2, 3 |
| Guests cannot see the directory or browse other teams | 1 (`org_allows`), 3 (`list_teams`, `team_access`) |
| Email invites with address, role and teams; waiting for registration; verified address required; mail off → admin-verified accounts | 3, 4, 5 |
| Instance setting: who creates organizations | 2, 5, 8 |
| Instance-wide ban keeps memberships; "Suspended by instance" | 2 (`suspended_by_instance`), 8 (badge) |
| Audit log recorded from day one, append-only, kept 1 year | 2 (trigger, purge), 5 (maintenance), 8 (minimal view) |
| Workspace switcher, per-workspace sidebar, visible current workspace, tinted header | 7 |
| Organization settings: Members, Teams, Policies, Audit log | 8 (Danger zone in Phase 4) |
| Plan checks in one place; self-hosted ungated | 1, 5 |
| Deferred: team space content, restricted folders, lifecycle, policies beyond the default role, audit filters and CSV, logo, search toggle, capture picker | Phases doc |
