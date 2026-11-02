# Possible Defects

Findings from a code review of the whole repository on 2026-11-01, in priority order within each severity. The Ranking section orders all of them by severity and likelihood. Seven reviewers covered auth, content access, vaults, data integrity, the worker and deployment, the web data layer, and web pages. The list merges their overlapping reports.

Treat these as leads. The findings in the Fixed table were checked and fixed. The rest were found by reading code and have not been reproduced by running it. The app is not deployed, so none of this has affected real users.

**Severity**

- **Critical:** exploitable data loss, data exposure or auth bypass with no special access, or a defect that loses or corrupts data in ordinary use.
- **High:** a real security or data defect that needs some condition, or a core feature that does not work.
- **Medium:** incorrect behaviour with limited impact.
- **Low:** a minor edge case.

Registration is open by default (`migrations/008_platform_settings.sql`), so "any account" means anyone who can reach the instance.

## Fixed

| ID | Defect | Fix |
| --- | --- | --- |
| C1 | **Enrichment SSRF with read-back.** Bookmark URLs and the favicon and OG URLs a page named were fetched with no host check, following redirects. The body was stored and served from `/api/bookmarks/{id}/favicon`, so any account could read cloud metadata and internal services. | `18dcac3`. One client (`crates/worker/src/net_guard.rs`) filters addresses in the resolver, so DNS rebinding is covered, and checks every redirect. The link checker uses it too, which also closes its IPv6 literal, redirect and DNS bypasses. Cached assets must be images by content. |
| C2 | **Screenshots opened any URL in Chrome.** `file:///etc/passwd`, `http://meilisearch:7700/` and redirects inward were rendered, saved and readable from `/preview`. Raised from high because it is the same hole as C1 through another door. | `18dcac3`. Non-public URLs are never opened. Every request Chrome makes is paused and checked, including subframes, scripts and redirects. Checked in real Chrome: an iframe of `meilisearch:7700/health` renders with the guard off and is blank with it on. The production compose file gives Chrome no route to the database or search. |
| C3 | **Registering took share invites.** With no email verification, anyone could register an invited address and accept the invite. `"bob@x.com "` with a trailing space also passed the unique check and took invites from an existing user. | `2072150`. An invite to an address with no account now gives the owner a single-use link, needed to claim it. Emails are trimmed at creation and lookup, and attached invites cannot be reassigned. OIDC only links when the provider says `email_verified`. H1 remains. |
| C4 | **Import corrupted URLs.** `parse_netscape` kept `&amp;` in `HREF` and `TAGS`, so every URL with a query string was damaged on import, including the app's own exports. The dev database has no affected rows. | `529944a`. Attributes are unescaped, and `&amp;` is decoded last. |
| D1 | **First visit race on the default dashboard.** Parallel requests hit `dashboards_one_default_per_user` and returned 500. | `c2c91b5`. Conflict-safe insert, with a concurrent test. |
| H2 | **Category cycles hung the API.** | `32e5344`. Moves check for a cycle under a lock, tested with concurrent opposite moves. Recursive queries use `UNION`, and pooled connections have a 30 second statement timeout. |
| H3 | **Import could kill the process.** | `1ab86f1`. Imports are capped at 64 folders deep and 10,000 folders, parent loops are refused, and parsing runs off the async runtime. Done with M24, because a bigger body made this easier to reach. |
| H4 | **A tiny tall image aborted the API.** | `a82bcb2`. Previews crop first, so they only ever scale to the target size. |
| H5 | **A retention value bricked startup.** | `97a787d`. Limited to 1 to 3650 in the API and by a `CHECK`, with existing values clamped. The purge no longer panics. |
| H6 | **No rate limits, and Argon2 unbounded.** | `01b577a`. Failed logins are limited per address and account pair (10), per account (50) and per address (50) over 15 minutes. Registrations are limited to 10 per address, and wrong vault secrets to 10 per user. Password hashing runs four at a time. Counters are in memory and reset on restart. Password change and global passphrase change are not limited. |
| H7 | **Vault keys never expired on the server.** | `d398cc5`. Keys are held per login session, keyed by the token hash. API keys cannot unlock or read vaults. The user's auto-lock time is enforced on the server, and logout, password change and the hourly sweep of ended sessions drop keys. Checked end to end against a throwaway API: a second session and an API key stay locked, and a password change locks the session. |
| H8 | **Decrypted vault titles were written to activity.** | `f0cd48a`. Vault rows log the field name, and migration 035 scrubs existing rows. |
| H9 | **Vault hostnames were sent to Google.** | `a6924cb`. Vault cards and tiles never call the lookup. Ordinary bookmarks still do. |
| H11 | **OIDC had no state, nonce or PKCE.** | `a340eab`. State, nonce and a PKCE verifier ride in a ten-minute HttpOnly cookie scoped to `/api/auth/oidc`, and the callback checks the state and validates the ID token's signature, issuer, audience, expiry and nonce. Checked against a mock provider: a forged state or a missing cookie goes back to `/login` with no session. |
| H12 | **OIDC ignored closed registration.** | `a340eab`. A new identity gets an account only while registration is open, as with `register`. The first account on an empty instance is still allowed, as with `register`; M3 remains. Banned users are refused at the callback. |
| H13 | **OIDC sign-in did not work at all.** | `a340eab`. The provider's endpoints come from discovery, so `OIDC_ISSUER` is the issuer as the provider states it. The service worker leaves `/api/` navigations to the network. The app guard asks `/auth/me` when there is no stored token, so a cookie session gets in, and an expired cookie session goes to `/login` from any app page. Failures show on the login page. Checked end to end against a mock provider; not yet against Authentik or in a browser. |
| H15 | **Unauthenticated metrics leaked public-list slugs.** | `0eb6379`. Metrics are labelled by route template. `/api/metrics` is off unless `METRICS_TOKEN` is set, and then needs it as a bearer token. `/api/ready` returns booleans. |
| H16 | **A write invitee could take a bookmark.** | `d8eafad`. Someone else's bookmark can only be filed in lists of people who already hold it, and links the editor cannot write are kept. |
| H17 | **Dashboard pins bypassed shares.** | `306fa33`. A shared dashboard shows the owner's own non-vault pins, as before. Any other pin shows only to people who can read the bookmark, so it no longer re-shares a folder bookmark or outlives a revoke, and the image routes follow the same rule. A write invitee can only add bookmarks they can read, and only rearrange the owner's pins already there. Unreadable pins are kept on save, not deleted, so they come back if access returns. |
| H18 | **Public lists ignored `is_private`.** | `71638c8`. Private bookmarks stay off `/l/{slug}`. Bookmarks an invitee added to a public list still show, and an admin takedown of the owner does not reach them. |
| H19 | **Invitees kept working in the owner's Trash.** | `cef6cf8`. A non-owner gets NotFound for a trashed bookmark, and for a folder when it or any ancestor is trashed, so reads, edits, social actions and shared-folder lists all stop at the trash. Shared folder lists skip shares under a trashed folder, and a folder listing shows trashed bookmarks to their owner only. |
| H20 | **Saving a layout deleted pins the editor could not see.** | `a8e4bc7`. Trashed, locked vault and other members' vault pins are kept in the section at the same index, or the last one, and a request cannot add them. |
| H21 | **Select all in Explorer selected the whole library.** | `8fe6dba`. It reaches what the open folder or search shows, and nothing at the top level. |
| H23 | **HTML export dropped live bookmarks.** | `bc4e301`. A folder whose parent is not exported starts its own tree, a bookmark in no exported folder goes at the top, and a bookmark is listed under each of its folders. |
| H24 | **Duplicate merge could trash the only live copy.** | `024aba9`. Duplicate detection leaves out trashed bookmarks. |
| H25 | **Clearing a field did nothing.** | `4ad76cd`. `null` now clears, and a missing field stays unchanged, for lists, bookmarks and profiles. |
| M1 | **Password change kept all sessions and API keys.** | `a729666`. A new password ends every other login session in the same transaction and keeps the one making the change. A checkbox also revokes every API key. Vault keys were already dropped (H7). |
| M7 | **Locked vault category names were served by Trash, activity and `GET /categories/{id}`.** | `a3bf7bb`. Trash and history say "Vault folder", migration 037 scrubs history, and PATCH returns 423 while the vault is locked. |
| M8 | **Move-in left plaintext tags, comments, audit titles and category activity.** | `caedb65`. Unused tags and social rows are deleted, and audit and list activity titles are replaced. |
| M16 | **Trashed bookmarks returned to search.** | `86c1241`. The search client removes a trashed bookmark instead of indexing it, and the backfill skips trash. |
| M18 | **Shared views showed the owner's other folder names and paths.** | `d8eafad`. Non-owners see only lists they can read, with paths cut at the highest one they can read. |
| M19 | **Email local part leaked to anonymous visitors and others.** | `24d2c63`. Public lists, followers, comments, notifications and block lists show a generic label. People shared with directly still see the local part. |
| M20 | **Likes and comments failed below a share root, because only direct shares counted.** | `HASH`. A share on any ancestor folder counts, for folders and for bookmarks in them. |
| M22 | **Enrichment concurrency was per call.** | `8862c1e`. One process-wide semaphore of four slots, and trashed bookmarks are not enriched. |
| M24 | **Documented upload limits were unreachable.** | `1ab86f1`. Avatars, previews and imports have their own body limits. |
| M25 | **Retention only ran at startup and only removed rows.** | `de15ca7`. Runs hourly and removes search documents and preview files. A trigger removes social rows on hard delete, and migration 036 clears orphans. |
| M30 | **Exports were not a full backup.** | `bc4e301`. Linkdean JSON version 2 keeps every folder, the favourite, unread, pinned and private flags, created and updated times, and dashboards. Version 1 still imports and newer versions are refused. Netscape import reads `<DD>`. |
| M31 | **Hot lists were unbounded with N+1 queries, including the anonymous public list.** | `43b23b8`. Views are built in a batch: tags, flags, folder links, shared-with labels, owner names and the owner's folders load once per list, not per bookmark. A public list shows the newest 500 and reports the total. Paging the library list and batching shared access checks are on the roadmap backlog. |
| M32 | **Known default secrets in the production stack.** | `bdef6d8`. The production compose file requires the secrets, and the API refuses published values unless `ALLOW_DEFAULT_SECRETS` is set. |
| M33 | **API ran as root, with no restart policy.** | `bdef6d8`. The image runs as UID 10001, and `db`, `meilisearch` and `api` restart unless stopped. |
| M34 | **No 401 handling.** | `245f56e`. Expired sessions clear the token and go to `/login`. A wrong password stays a plain 401. |
| M35 | **Login did not clear the previous user's cache, tabs were not told about logout.** | `a340eab`. Login and register clear the query cache. Signed-in tabs follow sign-out and sign-in in other tabs. A failed logout keeps the session and says so, and logout expires the session cookie. |
| M39 | **The permanent-delete warning read the filtered list, so an irreversible delete said it could be restored.** | `33aac2f`. The single-delete warning finds the bookmark wherever it was opened from, and the bulk warning reads the selected items. Selection drops what leaves the screen on a new search, folder or filter, so bulk actions only reach what is shown. |
| M41 | **Search input lost a trailing space.** | `bcb6698`. The box is no longer reset to the trimmed URL query. |
| M42 | **Search in Explorer on the Vaults page showed nothing.** | `820c6de`. Explorer reads the same matches as the grid. |
| M50 | **No confirmation on Replace tags (even when empty), Rotate public link, Replace recovery key and comment delete.** | `e3d0fc5`. Each now asks first and says what is lost. Replace tags needs at least one tag. The drawers behind these dialogs ignore Esc while the dialog is open. |
| L3 | **Expired sessions were never purged.** | `de15ca7`. The hourly maintenance task purges them. |
| L5 | **Authors could not delete comments after losing access, and owners could not delete comments on their items.** | `HASH`. Authors may always delete their own comments, and the owner of a bookmark, folder or dashboard may delete any comment on it. Others get 403. |
| L14 | **Worker failures were silent.** | `48fdd47`. Enrichment warns when a bookmark cannot be loaded, the page fetch fails or is refused, a reindex fails, a task panics, or the backfill cannot list rows. It logs "enriched" only when page metadata was saved. |
| L17 | **Public list previews returned 401 for anonymous visitors.** | `8bfb03d`. `/api/public/lists/{slug}/bookmarks/{id}/preview`, `/favicon` and `/og` serve the images of bookmarks on that list to anyone, and the public list points at them. |
| L20 | **Held Enter submitted repeatedly.** | `a340eab`. Login and Create key return early while a request is pending. |
| L24 | **SPA deep links returned a 404 status with the app's HTML.** | `a7c3bc1`. Client routes such as `/library` get the app shell with 200. Unknown `/api/` paths now return a JSON 404 instead of the app shell. Checked against a throwaway API. |
| L26 | **A button was nested in a button.** | `10bc9e1`. The folder menu is outside the folder tile button. |

Also closed by these fixes: stored XSS through cached favicons and OG images. Assets must now be images by content, and `/favicon` and `/og` send `nosniff` and a sandbox policy, which also covers files cached before the fix.

## Ranking

Every open defect, ranked by severity and likelihood. The detail for each ID is in the tables below.

These numbers are judgement calls, not measurements. Nothing here was reproduced by running it.

- **Probability:** the chance the defect causes real harm in the first year of a public instance with open registration and a few dozen users. It includes the chance the defect is not real.
- **Impact:** 1 is cosmetic, 2 is an annoyance, 3 is limited data exposure or a broken feature, 4 is permanent data loss or wider exposure, and 5 takes the whole instance down or hands it over.
- **Score:** impact × probability.
- **Trigger:** U is ordinary use, A needs a deliberate attacker, and C depends on deployment configuration. Closing registration cuts most A rows sharply. C rows depend on how the app is deployed.
- **Code read:** Yes means the cited code was reread for this ranking. No means the rating rests on the review report alone.
- **Tiers:** Tier 1 is a score of 2 or more, or impact 5 with at least 20% probability, because one such event takes every user down. Tier 2 is a score of 1 or more, Tier 3 is 0.3 or more, and Tier 4 is the rest.

Ratings changed by rereading the code:

- **M48** is much less likely than its row suggests. The last keystroke wins, so a wrong value only sticks if a request arrives out of order or the API restarts mid-typing.

### Tier 1: fix before any public deployment

All fixed. See the Fixed table.

### Tier 2: fix before the first public release

All fixed. See the Fixed table.

### Tier 3: fix when working nearby

| Rank | ID | Defect | Impact | Probability | Score | Trigger | Code read |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | M44 | The vault unlock modal reopens on every search, taking focus mid-typing | 2 | 40% | 0.80 | U | No |
| 2 | M49 | The invite gate is replaced after the first typed character, losing focus | 2 | 40% | 0.80 | U | No |
| 3 | L15 | Link checks churn `updated_at`, `put_dashboard` uses the pool inside a transaction | 1 | 80% | 0.80 | U | No |
| 4 | H22 | Purging a parent deletes live children | 4 | 15% | 0.60 | U | No |
| 5 | M26 | JSON import flattens nesting when a child sorts before its parent | 3 | 20% | 0.60 | U | No |
| 6 | M28 | Import is partial on failure | 3 | 20% | 0.60 | U | No |
| 7 | M27 | Path-keyed import turns `AC/DC` into `DC` and merges same-name siblings | 2 | 30% | 0.60 | U | No |
| 8 | M37 | A section with a blank title is silently dropped with its pins | 2 | 30% | 0.60 | U | No |
| 9 | M43 | Grid view ignores `folder`, so a palette jump changes nothing and can disable Add | 2 | 30% | 0.60 | U | No |
| 10 | M45 | Esc closes the modal and the parent drawer together, losing drafts | 2 | 30% | 0.60 | U | No |
| 11 | M9 | Moving a followers or members category into a vault returns 500 | 2 | 30% | 0.60 | U | No |
| 12 | L12 | A deleted preview comes back at the next backfill | 1 | 60% | 0.60 | U | No |
| 13 | M14 | Fork copies trashed content and is not transactional | 2 | 25% | 0.50 | U | No |
| 14 | M21 | Response bodies are read fully before any size check | 2 | 25% | 0.50 | A | No |
| 15 | H1 | Email addresses are never verified | 3 | 15% | 0.45 | A | Yes |
| 16 | M23 | No dimension limit on image decode | 3 | 15% | 0.45 | A | No |
| 17 | M29 | Multi-step bookmark writes have no transaction | 3 | 15% | 0.45 | U | No |
| 18 | M36 | Bulk add overwrites from a client snapshot | 3 | 15% | 0.45 | U | No |
| 19 | M6 | Sensitive vault actions need no passphrase, some while locked | 3 | 15% | 0.45 | A | No |
| 20 | L1 | Login timing reveals which emails exist | 1 | 40% | 0.40 | A | No |
| 21 | L19 | Esc in any overlay wipes the selection | 1 | 40% | 0.40 | U | No |
| 22 | H10 | Vault move leaves plaintext in search | 3 | 10% | 0.30 | U | Yes |
| 23 | M10 | Move crosses vault and owner boundaries, so deleting the other tree cascades into this one | 3 | 10% | 0.30 | U | No |
| 24 | M4 | Session token in localStorage undoes `HttpOnly` | 3 | 10% | 0.30 | A | No |
| 25 | M5 | Argon2 at the OWASP floor with an 8-byte minimum | 3 | 10% | 0.30 | A | No |
| 26 | L13 | The dev stack publishes known-credential services on all interfaces, sharing volumes with production | 2 | 15% | 0.30 | C | No |
| 27 | L16 | Spinner forever on a 5xx or network failure, with no Sign out | 2 | 15% | 0.30 | U | No |
| 28 | L6 | Nested shares show write where read is enforced | 2 | 15% | 0.30 | U | No |
| 29 | M11 | In-flight enrichment leaves assets for a row just moved to a vault | 2 | 15% | 0.30 | U | No |
| 30 | M13 | Dashboard blocker is not owner-scoped | 2 | 15% | 0.30 | U | No |
| 31 | M15 | Invitees can view and edit trashed dashboards | 2 | 15% | 0.30 | U | No |
| 32 | M17 | Owners cannot see or revoke a rejected invite | 2 | 15% | 0.30 | U | No |
| 33 | M47 | Finishing onboarding can bounce back, depending on timing | 2 | 15% | 0.30 | U | No |
| 34 | L18 | After cancelling a crop, the same file cannot be picked again | 1 | 30% | 0.30 | U | No |
| 35 | L28 | The test "loads API images with the page token" failed once under load and passed on rerun | 1 | 30% | 0.30 | U | No |
| 36 | M46 | Bulk Move to vault is offered for shared items the API refuses, failing the whole batch | 1 | 30% | 0.30 | U | No |

### Tier 4: leave unless it comes up

| Rank | ID | Defect | Impact | Probability | Score | Trigger | Code read |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 37 | L11 | Trashed vault items appear in vault search | 1 | 25% | 0.25 | U | No |
| 38 | H26 | Chrome 124 runs with `--no-sandbox` | 4 | 5% | 0.20 | A | Yes |
| 39 | M3 | Bootstrap admin is racy | 4 | 5% | 0.20 | A | No |
| 40 | H14 | One registration invite creates many accounts | 2 | 10% | 0.20 | A | Yes |
| 41 | M40 | Failed auto-lock reports locked | 2 | 10% | 0.20 | U | No |
| 42 | L22 | Bulk failures carry no detail | 1 | 20% | 0.20 | U | No |
| 43 | L25 | No `.dockerignore`, so local `node_modules` can overwrite the image's | 1 | 20% | 0.20 | C | No |
| 44 | L7 | A write invitee's tag edit moves tags out of the owner's namespace | 1 | 15% | 0.15 | U | No |
| 45 | M48 | Trash retention saves per keystroke, so typing 365 can store 3 and a later purge deletes items older than 3 days | 4 | 3% | 0.12 | U | Yes |
| 46 | L2 | A local account can block a victim's first OIDC login | 2 | 5% | 0.10 | C | No |
| 47 | L10 | An invitee-created bookmark blocks a move with a bare 404 | 1 | 10% | 0.10 | U | No |
| 48 | L21 | The previous user's avatar is served from HTTP cache for up to an hour | 1 | 10% | 0.10 | U | No |
| 49 | L23 | Tags containing a comma cannot be filtered | 1 | 10% | 0.10 | U | No |
| 50 | L4 | Hidden users and blocks can be detected through counts and status codes | 1 | 10% | 0.10 | A | No |
| 51 | M12 | Vault moves take no row lock, so a concurrent PATCH can write plaintext into a vault row | 3 | 3% | 0.09 | U | No |
| 52 | M2 | Last-admin check is racy | 3 | 2% | 0.06 | U | No |
| 53 | M38 | An edit draft can be saved onto a different dashboard after losing access | 2 | 3% | 0.06 | U | No |
| 54 | L8 | Keys are not zeroized | 2 | 2% | 0.04 | A | No |
| 55 | L9 | No AAD on ciphertext | 2 | 1% | 0.02 | A | No |
| 56 | L27 | URLs have no scheme check | 1 | 2% | 0.02 | A | No |

## High

| ID | Where | Defect | Suggested fix |
| --- | --- | --- | --- |
| H1 | `crates/db/src/repo/shares.rs:257`, `users.rs:386` | **Email addresses are never verified.** An attacker who registers `ceo@corp`, or changes their email to it, before the owner shares with that address receives the invite directly. C3 closes claiming after the fact, not this. | Email verification, which needs the SMTP roadmap item. Until then, show the invitee's display name before sharing. |
| H10 | `crates/api/src/routes/vault/mod.rs:266`, `routes/search/mod.rs:53` | **Vault move leaves plaintext in search.** Meilisearch cleanup is best-effort after commit, and search does not recheck Postgres, so a failed delete leaves the plaintext document returned while locked. | Outbox with retries, or recheck hits against `is_vault` and `deleted_at`. |
| H14 | `crates/api/src/routes/auth/mod.rs:67-92`, `crates/db/src/repo/invites.rs:150,169` | **One registration invite creates many accounts.** Concurrent registers all pass the check, and the user row survives a failed consume. | Claim the invite and insert the user in one transaction. |
| H22 | `crates/db/src/repo/categories.rs:465`, `trash.rs:124`, `migrations/001_init.sql:29` | **Purging a parent deletes live children.** Restoring only the child, or creating under a trashed parent, leaves a live folder that a later purge cascades away. | Reparent on restore, reject trashed parents, and reparent live children before purge. |
| H26 | `deploy/docker/docker-compose.yml:33`, `docker-compose.dev.yml:37` | **Chrome 124 runs with `--no-sandbox`.** A renderer exploit gets a shell in the container. Production is now isolated from the database and search, but the dev stack is not. | Maintained Chrome image, sandbox where possible, memory and pid limits, and the same networks in dev. |

## Medium

| ID | Where | Defect | Suggested fix |
| --- | --- | --- | --- |
| M2 | `crates/db/src/repo/users.rs:205` | Last-admin check is racy: two admins can demote each other, with no recovery short of SQL. | Check and update under the ban advisory lock. |
| M3 | `crates/api/src/routes/auth/mod.rs:54`, `users.rs:34` | Bootstrap admin is racy, and whoever registers first owns a fresh install. | Advisory lock, plus an optional `BOOTSTRAP_TOKEN`. |
| M4 | `apps/web/src/lib/auth/authToken.ts:8` | Session token in localStorage undoes `HttpOnly`. No XSS sink is known today. | Rely on the cookie in the browser. |
| M5 | `crates/core/src/crypto/mod.rs:155` | Argon2 at the OWASP floor with an 8-byte minimum, and parameters are not stored, so cost cannot be raised later. | Store parameters per wrap, raise to 64 MiB or more, and raise the minimum length. |
| M6 | `crates/api/src/routes/vault/mod.rs:161,206,228,239`, `vault_moves.rs:304` | Sensitive vault actions need no passphrase, some while locked: replacing the recovery key, changing the global passphrase, and purging locked items. | Require a passphrase or fresh unlock. Block purge and move on locked content. |
| M9 | `crates/db/src/repo/vault_moves.rs:343`, `migrations/026_profile_visibility.sql:14` | Moving a followers or members category into a vault returns 500. | Set `visibility='private'`, or add a blocker. |
| M10 | `crates/db/src/repo/categories.rs:280` | Move crosses vault and owner boundaries, so deleting the other tree cascades into this one. | Require same owner and same `vault_id`. |
| M11 | `crates/worker/src/lib.rs:74-123`, `crates/api/src/routes/bookmarks/mod.rs:256,270` | In-flight enrichment leaves assets for a row just moved to a vault, and `/favicon` and `/og` have no vault check. | Recheck after each write, and refuse vault rows on those routes. |
| M12 | `crates/db/src/repo/vault_moves.rs:210,264`, `bookmarks.rs:950` | Vault moves take no row lock, so a concurrent PATCH can write plaintext into a vault row. | `FOR UPDATE`, a version check, and `is_vault=FALSE` on plain updates. |
| M13 | `crates/db/src/repo/vault_moves.rs:169` | Dashboard blocker is not owner-scoped: someone else's stale pin blocks a move forever and leaks that dashboard's title. | Delete foreign pins, and block only on owned dashboards. |
| M14 | `crates/db/src/repo/categories.rs:603` | Fork copies trashed content and is not transactional. | Filter `deleted_at`, and use one transaction. |
| M15 | `crates/db/src/repo/dashboard_shares.rs:67` | Invitees can view and edit trashed dashboards. | NotFound for non-owners when deleted. |
| M17 | `crates/db/src/repo/pending_shares.rs:489` | Owners cannot see or revoke a rejected invite, and the invitee can restore it at any time. | Let owners revoke, or make rejection final. |
| M21 | `crates/worker/src/lib.rs:52,299` | Response bodies are read fully before any size check, and titles are stored with no length limit. | Stream with a byte cap, and truncate. |
| M23 | `crates/core/src/avatar/mod.rs:28`, `preview_image/mod.rs:31` | No dimension limit on image decode: N uploads cost N × 512 MiB. | Read header dimensions first, and gate decodes. |
| M26 | `crates/core/src/import_export/mod.rs:177` | JSON import flattens nesting when a child sorts before its parent. | Topological sort. Error on an unresolved parent. |
| M27 | `crates/db/src/repo/import.rs:61`, `crates/core/src/import_export/mod.rs:413` | Path-keyed import turns `AC/DC` into `DC` and merges same-name siblings. | Key by node, not joined names. |
| M28 | `crates/api/src/routes/import_export/mod.rs:62` | Import is partial on failure: the job stays `running` and rows are missing from search. | Background job with error status, and batch reindex. |
| M29 | `crates/db/src/repo/bookmarks.rs:40,609,928` | Multi-step bookmark writes have no transaction. A failed PATCH half-applies, a NUL in a tag wipes all tags, and concurrent edits are lost. | One transaction with `FOR UPDATE`. Validate first. |
| M36 | `apps/web/src/hooks/bookmarks/useBulkBookmarks.ts:59` | Bulk add overwrites from a client snapshot, and a missing id wipes tags and categories. | Server-side add, or fail on a miss. |
| M37 | `apps/web/src/services/dashboards/dashboardDraft.ts:49` | A section with a blank title is silently dropped with its pins. | Block save with a validation error. |
| M38 | `apps/web/src/hooks/dashboards/useDashboardPage.ts:23` | An edit draft can be saved onto a different dashboard after losing access. | Bind the draft to its dashboard id. |
| M40 | `apps/web/src/hooks/prefs/usePrefs.ts:76` | Failed auto-lock reports locked, and leaves decrypted data on screen. | Keep the unlocked state, retry, and always drop caches. |
| M43 | `apps/web/src/hooks/library/useLibraryList.ts:111` | Grid view ignores `folder`, so a palette jump changes nothing and can disable Add. | Filter by folder and show a chip, or force Explorer. |
| M44 | `apps/web/src/hooks/library/useLibraryBookmarkEditor.ts:53` | The vault unlock modal reopens on every search, taking focus mid-typing. | Remember dismissal for the session. |
| M45 | `apps/web/src/components/avatar/AvatarCropModal.tsx:77`, `pages/settings/ModerationDrawer.tsx` | Esc closes the modal and the parent drawer together, losing drafts. | Use `closeOnEscape={!childOpen}` or stacks. |
| M46 | `apps/web/src/pages/library/LibraryBulkBar.tsx:52` | Bulk Move to vault is offered for shared items the API refuses, failing the whole batch. | Send owned ids only, or disable with a reason. |
| M47 | `apps/web/src/hooks/auth/useAuth.ts:49`, `pages/onboarding/OnboardingPage.tsx:80` | Finishing onboarding can bounce back, depending on timing. Not reproduced. | Update the cache from the response. |
| M48 | `apps/web/src/pages/settings/GeneralSettingsPanel.tsx:47` | Trash retention saves per keystroke, so typing 365 can store 3 and a later purge deletes items older than 3 days. | Draft locally and save on blur. |
| M49 | `apps/web/src/pages/register/RegisterPage.tsx:93` | The invite gate is replaced after the first typed character, losing focus. | Gate on `search.invite`. |

## Low

| ID | Where | Defect | Suggested fix |
| --- | --- | --- | --- |
| L1 | `crates/db/src/repo/users.rs:493` | Login timing reveals which emails exist. | Verify against a dummy hash. |
| L2 | `crates/api/src/routes/auth/mod.rs:289` | A local account can block a victim's first OIDC login. | Email verification, or an approved link flow. |
| L4 | `crates/api/src/routes/users/mod.rs:211`, `follows.rs:42`, `shares.rs:274` | Hidden users and blocks can be detected through counts and status codes. | Gate through `require_viewable_profile`, and use uniform responses. |
| L6 | `crates/db/src/repo/shares.rs:155` | Nested shares show write where read is enforced. | Compute the nearest grant. |
| L7 | `crates/db/src/repo/bookmarks.rs:968` | A write invitee's tag edit moves tags out of the owner's namespace. | Use the bookmark owner's namespace. |
| L8 | `crates/core/src/models/mod.rs:755`, `crates/api/src/state/mod.rs:87`, `crates/core/src/crypto/mod.rs:154` | Keys are not zeroized, and `VaultKeyring` derives `Debug` over raw keys. | `Zeroizing`, redacted `Debug`, and `Arc` the ring. |
| L9 | `crates/core/src/crypto/mod.rs:99,169` | No AAD on ciphertext, and `public_key` is unauthenticated. Needs database write access. | AAD of vault and bookmark id. Verify the key on open. |
| L10 | `crates/db/src/repo/vault_moves.rs:66,206,261` | An invitee-created bookmark blocks a move with a bare 404, and `is_private` is lost on a round trip. | Blocker kind, and add the field to the payload. |
| L11 | `crates/db/src/repo/vault.rs:15`, `vault_moves.rs:46,395` | Trashed vault items appear in vault search, and move-out re-indexes trashed rows. | Filter `deleted_at`. |
| L12 | `crates/api/src/routes/bookmarks/mod.rs:231`, `crates/db/src/repo/bookmarks.rs:1217` | A deleted preview comes back at the next backfill, and permanent failures retry every boot. | Suppression flag or attempted-at marker. |
| L13 | `deploy/docker/docker-compose.dev.yml:13,30,71,97` | The dev stack publishes known-credential services on all interfaces, sharing volumes with production. | Bind to `127.0.0.1`. |
| L15 | `crates/db/src/repo/bookmarks.rs` (`record_link_check`), `dashboards.rs` (`put_dashboard`), `tags.rs` | Link checks churn `updated_at`, `put_dashboard` uses the pool inside a transaction, and tag rows are never reaped. | Separate column, use the transaction, and reap. |
| L16 | `apps/web/src/pages/banned/BannedPage.tsx:27` | Spinner forever on a 5xx or network failure, with no Sign out. | Error and retry state. |
| L18 | `apps/web/src/components/bookmark/BookmarkPreviewField.tsx:133` | After cancelling a crop, the same file cannot be picked again. | Add `resetRef`. |
| L19 | `apps/web/src/hooks/library/useLibraryPageChrome.ts:60` | Esc in any overlay wipes the selection. | Skip when an overlay is open. |
| L21 | `apps/web/src/hooks/avatar/useAvatarImage.ts:18`, `crates/api/src/routes/users/mod.rs:171` | The previous user's avatar is served from HTTP cache for up to an hour. | Use `/users/{id}/avatar`, or `no-store`. |
| L22 | `apps/web/src/hooks/library/useLibrarySelection.ts:13,79` | Bulk failures carry no detail, and selection clears on partial failure. | Keep failed ids selected, with the reason. |
| L23 | `apps/web/src/lib/library/librarySearch.ts:37` | Tags containing a comma cannot be filtered. | Repeat the param. |
| L25 | Repository root | No `.dockerignore`, so local `node_modules` can overwrite the image's. | Add one. |
| L27 | `crates/db/src/repo/bookmarks.rs` (`create_bookmark`) | URLs have no scheme check. React 19 blocks `javascript:` links, and the worker refuses non-web URLs, so this is defence in depth. | Reject non-http(s) at create, update and import. |
| L28 | `apps/web/src/hooks/images/useApiImage.test.ts` | The test "loads API images with the page token" failed once under load and passed on rerun. | Check for a timing dependency. |

## Behaviour changes from the fixes

- The production stack no longer starts until `POSTGRES_PASSWORD`, `MEILI_MASTER_KEY` and `SESSION_SECRET` are set. The API refuses published default values unless `ALLOW_DEFAULT_SECRETS` is set, which only the dev stack and `.env.example` do.
- Behind a reverse proxy, set `TRUST_FORWARDED_FOR=true`. Otherwise every client shares the proxy's address for login limits.
- A stranger can lock an account out of password login for 15 minutes with 50 failed attempts. That is the cost of the per-account limit.
- Imports deeper than 64 folders, or with more than 10,000 folders, are refused.
- API clients that sent `null` to mean "unchanged" now clear the field.
- URLs on private addresses, including `.local`, `.lan` and single-label names, no longer get metadata, previews or link checks. This affects homelab bookmarks. An allowlist setting could restore it if wanted.
- A person with no account now needs the invite link. Registering with the invited address without the link attaches nothing. If they do that, the owner can invite them again to reach their account.
- Invites created before migration 033 have no link. Invite again to get one.
- Unlocking a vault opens it for that login session only. Before, it opened it for every session and API key of the user. API keys get `403` from the unlock routes and never see vault content.
- The server now locks vaults after the auto-lock time with no request using them, and on a password change.
- Cached favicons and OG images must be PNG, JPEG, GIF, WebP, ICO or SVG by content. Anything else falls back to the remote URL on the card.
- `OIDC_ISSUER` must be the issuer exactly as the provider's discovery document states it, trailing slash included. OIDC sign-in no longer creates accounts while registration is closed.
- A password change signs out every other session.
