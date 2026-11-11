# Organizations and teams

Requirements agreed with the product owner on 2026-11-03, in a staff-engineer interview. This is the product model, not yet an implementation plan. It is built after outbound email parts B (password reset) and C (email verification), because invites must only reach verified addresses.

## Vocabulary

Fixed now and used in the UI, API and docs, in American spelling.

| Concept | Term | Not |
| --- | --- | --- |
| The customer account | **Organization** | Company, Tenant, Account |
| Where you are working | **Workspace**: "Personal" or an organization | Context, Profile |
| A group of people | **Team** | Group, Department |
| A team's content area | **Team space** (in the UI, the team's name) | Room, Channel, Vault |
| People in an organization | **Members**; outsiders are **Guests** | Users, Collaborators |
| A folder with narrowed access | **Restricted folder** | Private folder |
| Bringing people in | **Invite** | Add |
| The instance operator | **Instance admin** | Super admin, Root |
| Commercial tiers | **Plans** | Tiers, Packages |

## Ownership model

- An instance hosts many organizations. A user can belong to several, and to none.
- **Personal stays exactly as today**: library, vaults, and sharing with individual people. A solo user sees no organization or team UI beyond a "Create an organization" entry in the account menu.
- Content has one of two owners: a **user** (personal) or an **organization**.
- Every team belongs to exactly one organization. A family or freelancer who wants a team creates a small organization.
- Personal content is never exposed to an organization or team by membership. It leaves the personal library only when its owner shares it (as today) or moves it into a team space, after a warning that the organization will own it.
- Organization owners and admins have no view into members' personal libraries.
- Every organization has at least one team. A **default team** is created with it and cannot be deleted.

## Where organization content lives

- Each team has a **team space**: folders, bookmarks, tags, dashboards.
- Access comes from team membership and the team role.
- A folder can be **restricted** to some members or teams of its space. Restrictions only narrow access, never widen it; moving content to another team is how others get it. Organization owners and admins always see restricted folders.
- Effective access to a folder = the team role, narrowed by the folder's restriction.

## Roles

### Organization roles

| Role | Can |
| --- | --- |
| **Owner** | Everything: billing and plan, delete or transfer the organization, promote admins. At least one per organization; the last owner cannot leave or be demoted. |
| **Admin** | Invite and remove members, create and manage teams, organization settings and policies, see every team space. No billing, no deleting the organization. |
| **Member** | Join or be added to teams, work in their teams' spaces, create teams when policy allows. |
| **Guest** | Only the specific teams they are added to. Cannot see the member directory or browse other teams. Not a paid seat on SaaS. |

### Team roles

| Role | Can |
| --- | --- |
| **Maintainer** | Everything an Editor can, plus add and remove team members, change their team roles, set folder restrictions, rename or archive the team. |
| **Editor** | Add, edit, move and delete bookmarks, folders and dashboards; comment and like. |
| **Viewer** | Read, search, open, comment and like. |

- Organization owners and admins act as Maintainer in every team without being listed.
- Every member joins the default team as Editor; admins can change that default to Viewer.
- Guests never join the default team automatically.

## Joining an organization (v1)

- **Email invites only**: an admin picks the address, organization role and teams.
- An invite to an address without an account waits until someone registers with that address, as pending folder-share invites do today.
- Accepting requires a verified email address (part C). With mail off, only accounts an admin created or approved can accept.

## Team space features (v1)

| Feature | v1 |
| --- | --- |
| Bookmarks, folders, folder restrictions | Yes |
| Tags | Yes, per team space |
| Dashboards | Yes |
| Likes, comments, ratings | Yes |
| Library health (broken links, duplicates) | Yes, per space, run by maintainers |
| Import and export | Yes, by maintainers |
| Activity feed | Yes, scoped to the team |
| Vaults | No (backlog: team vaults) |
| Public share links | Off by default; an organization policy lets admins allow them |

## Lifecycle

| Event | Behavior |
| --- | --- |
| A member leaves or is removed | Access ends at once. Content they added stays with the organization, attributed to them as "Former member". Their personal library is untouched. |
| The last owner tries to leave | Blocked until someone else is made owner. |
| A team is archived | Read-only, hidden from the switcher, can be restored. |
| A team is deleted | Owners and admins only; never the default team. Content goes to the organization's Deleted items for the retention period (default 30 days), then is purged. |
| The organization is deleted | Owners only, confirmed by typing the name. 30-day grace period with restore, then a hard purge. Every owner is emailed. Admins can export before or during the grace period. |
| Content moves personal → team | The organization owns it, after the warning. |
| Content moves team → personal | Copy only, never move. Admins can turn copying off. |
| A user deletes their account | As leaving every organization. Blocked while they are the last owner of one. |

## Instance admins and organizations

| | Instance admin | Organization owner or admin |
| --- | --- | --- |
| Create organizations | Instance setting: anyone (default) or instance admins only | — |
| See organization content | No. Names, member counts and storage only. | Their own organization |
| Suspend or delete an organization | Yes, for abuse or non-payment. Suspended is read-only and blocks sign-in to it. | Delete, with the grace period |
| Members | No, except an instance-wide ban (below) | Yes |
| Moderation of public content | Yes, as today | Can turn public sharing off by policy |
| Organization policies | — | Yes: public sharing, who creates teams, team → personal copy, retention |

- An **instance-wide ban** blocks sign-in completely but keeps every organization membership, role and team, so access returns unchanged when the ban ends. Organization admins see the member as "Suspended by instance" and may still remove them. On SaaS the seat stays billed until the organization removes them.
- The docs say plainly that a self-hosted operator can read organization data from the database. The product promise is "no in-app access", not "technically impossible"; only client-held keys would make it impossible.

## Audit log (v1)

- Records membership (invites, joins, removals, role changes), teams (create, archive, delete, maintainers), policies, folder restrictions, organization deletion and restore, and content deletions and moves into and out of the organization.
- Owners and admins see it in Organization settings → Audit log, filtered by person, action and date.
- Kept 1 year. CSV export. Append-only: the app never edits or deletes entries except by age.
- Events are recorded from day one even if the view is minimal, so the log has no gap.

## Interface

- Solo users: unchanged.
- Members of an organization get a **workspace switcher** at the top of the sidebar ("Personal", "Acme Inc."). Each workspace has its own sidebar, search and dashboards. The current workspace is always visible: the switcher shows the organization's name and logo, and the header is subtly tinted.
- Inside an organization, the sidebar lists the user's teams, each expanding to its folder tree, the default team first. Restricted folders show a lock.
- Organization settings for owners and admins: Members, Teams, Policies, Audit log, Danger zone.
- Search covers the current workspace by default, with an "all my workspaces" toggle.
- Capture (paste, the future browser extension) saves to the current workspace, with a picker.

## SaaS plans

Self-hosted has every feature, with no plan gates. Plan checks live in one place in the code so the SaaS can gate features without scattering checks.

| Plan | For | Includes |
| --- | --- | --- |
| Free | Individuals, families | Personal library, 1 vault; 1 organization with up to 3 members and 2 teams |
| Pro | Individuals | Unlimited vaults; later, more storage for archives |
| Team (per seat) | Small companies | Unlimited members and teams, roles, guests, folder restrictions, basic audit log |
| Business (per seat) | Growing companies | Adds verified domains, invite links, organization policies, audit log export |
| Enterprise (contract) | Large companies | Adds SSO/SAML, SCIM, custom roles, audit log streaming, team vaults, SLA |

- Guests are free, up to 5 per paid seat.
- A downgrade never locks anyone out of their own data: over-limit vaults stay unlockable and exportable; only creating new ones is blocked.
- Pricing models, and whether there are one or several paid personal plans, are decided when the SaaS release is planned.

## Backlog (after v1)

- Invite links (role, expiry, use limit; off by default).
- Verified domains: DNS TXT proof, then auto-join or join-on-request for matching verified addresses.
- SSO (SAML / per-organization OIDC) and SCIM provisioning.
- Team vaults: shared keys, rotation when members leave, recovery.
- Audit log: every content edit and restricted-folder view, configurable retention, SIEM or webhook streaming, hash-chained tamper evidence.
- Custom roles and a Billing manager role, when a customer needs them.
- SaaS pricing models and the paid personal plans.
