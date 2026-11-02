# Possible Defects

Findings from a code review of the whole repository on 2026-11-01, in priority order within each severity. The Ranking section orders all of them by severity and likelihood. Seven reviewers covered auth, content access, vaults, data integrity, the worker and deployment, the web data layer, and web pages. The list merges their overlapping reports.

Treat these as leads. Only the four critical findings were checked against the code and fixed. The rest were found by reading code and have not been reproduced by running it. The app is not deployed, so none of this has affected real users.

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
- **H5** cannot be reached by the normal form, which stops at 3650. It needs a direct API call.
- **H15** only leaks slugs of public lists someone has visited since the last restart.
- **H11 and H12** rate low only while OIDC is unused. Recheck them when H13 is fixed.
- **M33** makes every crash in Tier 1 worse, because the API does not come back by itself.

### Tier 1: fix before any public deployment

| Rank | ID | Defect | Impact | Probability | Score | Trigger | Code read |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | H25 | Clearing a field does nothing | 3 | 95% | 2.85 | U | Yes |
| 2 | H6 | No rate limits, and Argon2 is unbounded | 4 | 70% | 2.80 | A | Yes |
| 3 | H9 | Vault hostnames sent to Google | 3 | 90% | 2.70 | U | Yes |
| 4 | H8 | Decrypted vault titles written to activity | 3 | 80% | 2.40 | U | Yes |
| 5 | M33 | API runs as root | 3 | 80% | 2.40 | U | Yes |
| 6 | M34 | No 401 handling | 3 | 80% | 2.40 | U | No |
| 7 | M24 | Documented upload limits are unreachable | 3 | 70% | 2.10 | U | Yes |
| 8 | H2 | Category cycles hang the API | 5 | 40% | 2.00 | A | Yes |
| 9 | M32 | Known default secrets in the production stack | 5 | 40% | 2.00 | C | No |
| 10 | H18 | Public lists ignore `is_private` | 4 | 50% | 2.00 | U | Yes |
| 11 | H21 | Select all in Explorer selects the whole library | 4 | 50% | 2.00 | U | Yes |
| 12 | H4 | Tiny tall image aborts the API | 5 | 25% | 1.25 | A | Yes |
| 13 | H5 | Retention value bricks startup | 5 | 20% | 1.00 | A | Yes |

### Tier 2: fix before the first public release

| Rank | ID | Defect | Impact | Probability | Score | Trigger | Code read |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 14 | M19 | Email local part leaks to anonymous visitors and others | 3 | 60% | 1.80 | U | No |
| 15 | M22 | Enrichment concurrency is per call, so a burst opens hundreds of Chrome tabs | 3 | 60% | 1.80 | U | No |
| 16 | M8 | Move-in leaves plaintext tags, comments, audit titles and category activity | 3 | 60% | 1.80 | U | No |
| 17 | M25 | Retention only runs at startup and only removes rows | 2 | 90% | 1.80 | U | Yes |
| 18 | H20 | Saving a layout deletes pins the editor cannot see | 4 | 40% | 1.60 | U | Yes |
| 19 | H15 | Unauthenticated metrics leak public-list slugs | 3 | 50% | 1.50 | A | Yes |
| 20 | L17 | Public list previews return 401 for anonymous visitors | 2 | 70% | 1.40 | U | No |
| 21 | H7 | Vault keys never expire on the server | 4 | 30% | 1.20 | A | Yes |
| 22 | H23 | HTML export drops live bookmarks | 3 | 40% | 1.20 | U | Yes |
| 23 | M30 | Exports are not a full backup | 3 | 40% | 1.20 | U | No |
| 24 | M31 | Hot lists are unbounded with N+1 queries, including the anonymous public list | 3 | 40% | 1.20 | U | No |
| 25 | M18 | Shared views show the owner's other folder names and paths | 2 | 60% | 1.20 | U | No |
| 26 | M41 | Search input loses a trailing space, joining the next word | 2 | 60% | 1.20 | U | No |
| 27 | H24 | Duplicate merge can trash the only live copy | 3 | 35% | 1.05 | U | Yes |
| 28 | H16 | Write invitee can take a bookmark | 4 | 25% | 1.00 | U | Yes |
| 29 | M16 | Trashed bookmarks return to search through any PATCH, boot backfill or vault move-out | 2 | 50% | 1.00 | U | No |
| 30 | M42 | Search in Explorer on the Vaults page always shows nothing | 2 | 50% | 1.00 | U | No |
| 31 | M7 | Locked vault category names are served by Trash, activity and `GET /categories/{id}` | 2 | 50% | 1.00 | U | No |
| 32 | L26 | A button is nested in a button | 1 | 100% | 1.00 | U | No |
| 33 | L3 | Expired sessions are never purged | 1 | 100% | 1.00 | U | No |

### Tier 3: fix when working nearby

| Rank | ID | Defect | Impact | Probability | Score | Trigger | Code read |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 34 | H13 | OIDC sign-in does not work at all | 3 | 30% | 0.90 | C | Yes |
| 35 | H17 | Dashboard pins bypass shares | 3 | 30% | 0.90 | U | Yes |
| 36 | H19 | Invitees keep working in the owner's Trash | 3 | 30% | 0.90 | U | Yes |
| 37 | M1 | Password change keeps all sessions and API keys | 3 | 30% | 0.90 | U | No |
| 38 | M35 | Login does not clear the previous user's cache, tabs are not told about logout | 3 | 30% | 0.90 | U | No |
| 39 | M39 | The permanent-delete warning reads the filtered list, so an irreversible delete says it can be restored | 3 | 30% | 0.90 | U | No |
| 40 | M50 | No confirmation on Replace tags (even when empty), Rotate public link, Replace recovery key and comment delete | 3 | 30% | 0.90 | U | No |
| 41 | L14 | Worker failures are silent | 1 | 90% | 0.90 | U | No |
| 42 | L24 | SPA deep links return a 404 status with the app's HTML | 1 | 90% | 0.90 | U | No |
| 43 | H3 | Import can kill the process | 4 | 20% | 0.80 | A | Yes |
| 44 | M20 | Likes and comments fail below a share root, because only direct shares count | 2 | 40% | 0.80 | U | No |
| 45 | M44 | The vault unlock modal reopens on every search, taking focus mid-typing | 2 | 40% | 0.80 | U | No |
| 46 | M49 | The invite gate is replaced after the first typed character, losing focus | 2 | 40% | 0.80 | U | No |
| 47 | L15 | Link checks churn `updated_at`, `put_dashboard` uses the pool inside a transaction | 1 | 80% | 0.80 | U | No |
| 48 | H22 | Purging a parent deletes live children | 4 | 15% | 0.60 | U | No |
| 49 | M26 | JSON import flattens nesting when a child sorts before its parent | 3 | 20% | 0.60 | U | No |
| 50 | M28 | Import is partial on failure | 3 | 20% | 0.60 | U | No |
| 51 | M27 | Path-keyed import turns `AC/DC` into `DC` and merges same-name siblings | 2 | 30% | 0.60 | U | No |
| 52 | M37 | A section with a blank title is silently dropped with its pins | 2 | 30% | 0.60 | U | No |
| 53 | M43 | Grid view ignores `folder`, so a palette jump changes nothing and can disable Add | 2 | 30% | 0.60 | U | No |
| 54 | M45 | Esc closes the modal and the parent drawer together, losing drafts | 2 | 30% | 0.60 | U | No |
| 55 | M9 | Moving a followers or members category into a vault returns 500 | 2 | 30% | 0.60 | U | No |
| 56 | L12 | A deleted preview comes back at the next backfill | 1 | 60% | 0.60 | U | No |
| 57 | M14 | Fork copies trashed content and is not transactional | 2 | 25% | 0.50 | U | No |
| 58 | M21 | Response bodies are read fully before any size check | 2 | 25% | 0.50 | A | No |
| 59 | H1 | Email addresses are never verified | 3 | 15% | 0.45 | A | Yes |
| 60 | M23 | No dimension limit on image decode | 3 | 15% | 0.45 | A | No |
| 61 | M29 | Multi-step bookmark writes have no transaction | 3 | 15% | 0.45 | U | No |
| 62 | M36 | Bulk add overwrites from a client snapshot | 3 | 15% | 0.45 | U | No |
| 63 | M6 | Sensitive vault actions need no passphrase, some while locked | 3 | 15% | 0.45 | A | No |
| 64 | L1 | Login timing reveals which emails exist | 1 | 40% | 0.40 | A | No |
| 65 | L19 | Esc in any overlay wipes the selection | 1 | 40% | 0.40 | U | No |
| 66 | H10 | Vault move leaves plaintext in search | 3 | 10% | 0.30 | U | Yes |
| 67 | M10 | Move crosses vault and owner boundaries, so deleting the other tree cascades into this one | 3 | 10% | 0.30 | U | No |
| 68 | M4 | Session token in localStorage undoes `HttpOnly` | 3 | 10% | 0.30 | A | No |
| 69 | M5 | Argon2 at the OWASP floor with an 8-byte minimum | 3 | 10% | 0.30 | A | No |
| 70 | L13 | The dev stack publishes known-credential services on all interfaces, sharing volumes with production | 2 | 15% | 0.30 | C | No |
| 71 | L16 | Spinner forever on a 5xx or network failure, with no Sign out | 2 | 15% | 0.30 | U | No |
| 72 | L6 | Nested shares show write where read is enforced | 2 | 15% | 0.30 | U | No |
| 73 | M11 | In-flight enrichment leaves assets for a row just moved to a vault | 2 | 15% | 0.30 | U | No |
| 74 | M13 | Dashboard blocker is not owner-scoped | 2 | 15% | 0.30 | U | No |
| 75 | M15 | Invitees can view and edit trashed dashboards | 2 | 15% | 0.30 | U | No |
| 76 | M17 | Owners cannot see or revoke a rejected invite | 2 | 15% | 0.30 | U | No |
| 77 | M47 | Finishing onboarding can bounce back, depending on timing | 2 | 15% | 0.30 | U | No |
| 78 | L18 | After cancelling a crop, the same file cannot be picked again | 1 | 30% | 0.30 | U | No |
| 79 | L28 | The test "loads API images with the page token" failed once under load and passed on rerun | 1 | 30% | 0.30 | U | No |
| 80 | L5 | Authors cannot delete comments after losing access | 1 | 30% | 0.30 | U | No |
| 81 | M46 | Bulk Move to vault is offered for shared items the API refuses, failing the whole batch | 1 | 30% | 0.30 | U | No |

### Tier 4: leave unless it comes up

| Rank | ID | Defect | Impact | Probability | Score | Trigger | Code read |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 82 | L11 | Trashed vault items appear in vault search | 1 | 25% | 0.25 | U | No |
| 83 | H12 | OIDC ignores closed registration | 4 | 5% | 0.20 | C | Yes |
| 84 | H26 | Chrome 124 runs with `--no-sandbox` | 4 | 5% | 0.20 | A | Yes |
| 85 | M3 | Bootstrap admin is racy | 4 | 5% | 0.20 | A | No |
| 86 | H14 | One registration invite creates many accounts | 2 | 10% | 0.20 | A | Yes |
| 87 | M40 | Failed auto-lock reports locked | 2 | 10% | 0.20 | U | No |
| 88 | L22 | Bulk failures carry no detail | 1 | 20% | 0.20 | U | No |
| 89 | L25 | No `.dockerignore`, so local `node_modules` can overwrite the image's | 1 | 20% | 0.20 | C | No |
| 90 | L7 | A write invitee's tag edit moves tags out of the owner's namespace | 1 | 15% | 0.15 | U | No |
| 91 | M48 | Trash retention saves per keystroke, so typing 365 can store 3 and a later purge deletes items older than 3 days | 4 | 3% | 0.12 | U | Yes |
| 92 | L2 | A local account can block a victim's first OIDC login | 2 | 5% | 0.10 | C | No |
| 93 | L10 | An invitee-created bookmark blocks a move with a bare 404 | 1 | 10% | 0.10 | U | No |
| 94 | L20 | Held Enter submits repeatedly, creating several sessions or API keys | 1 | 10% | 0.10 | U | No |
| 95 | L21 | The previous user's avatar is served from HTTP cache for up to an hour | 1 | 10% | 0.10 | U | No |
| 96 | L23 | Tags containing a comma cannot be filtered | 1 | 10% | 0.10 | U | No |
| 97 | L4 | Hidden users and blocks can be detected through counts and status codes | 1 | 10% | 0.10 | A | No |
| 98 | H11 | OIDC has no state, nonce or PKCE | 3 | 3% | 0.09 | C | Yes |
| 99 | M12 | Vault moves take no row lock, so a concurrent PATCH can write plaintext into a vault row | 3 | 3% | 0.09 | U | No |
| 100 | M2 | Last-admin check is racy | 3 | 2% | 0.06 | U | No |
| 101 | M38 | An edit draft can be saved onto a different dashboard after losing access | 2 | 3% | 0.06 | U | No |
| 102 | L8 | Keys are not zeroized | 2 | 2% | 0.04 | A | No |
| 103 | L9 | No AAD on ciphertext | 2 | 1% | 0.02 | A | No |
| 104 | L27 | URLs have no scheme check | 1 | 2% | 0.02 | A | No |

## High

| ID | Where | Defect | Suggested fix |
| --- | --- | --- | --- |
| H1 | `crates/db/src/repo/shares.rs:257`, `users.rs:386` | **Email addresses are never verified.** An attacker who registers `ceo@corp`, or changes their email to it, before the owner shares with that address receives the invite directly. C3 closes claiming after the fact, not this. | Email verification, which needs the SMTP roadmap item. Until then, show the invitee's display name before sharing. |
| H2 | `crates/db/src/repo/categories.rs:276`, `shares.rs:55,129,549` | **Category cycles hang the API.** `PATCH /categories/A {parent_id: A}` has no cycle check. Recursive CTEs and a `while` loop then never end, and with no `statement_timeout` a pool of 20 is exhausted. | Reject self or descendant parents under a lock. Use `UNION` or a depth limit. Set a statement timeout. |
| H3 | `crates/core/src/import_export/mod.rs:349`, `:99` | **Import can kill the process.** A JSON `parent_id` cycle loops forever on a tokio worker. A deep `<H3><DL>` stack uses O(depth²) memory. | Cycle and depth caps, a category cap, and parse in `spawn_blocking`. |
| H4 | `crates/core/src/preview_image/mod.rs:47` | **Tiny tall image aborts the API.** A 1×65535 PNG upscales to tens of GB. Combined with no restart policy (M33), the API stays down. | Reject extreme aspect ratios, or crop first and only downscale. |
| H5 | `crates/db/src/repo/trash.rs:110`, `users.rs:313`, `crates/api/src/main.rs:29` | **Retention value bricks startup.** Any user can set `trash_retention_days` near `i32::MAX`. The date subtraction panics in `main` on every boot. | Validate 1 to 3650 in the API with a DB `CHECK`, and use `checked_sub_signed`. |
| H6 | `crates/api/src/routes/auth/mod.rs:114`, `routes/vault/mod.rs:30`, `crates/db/src/repo/blocking.rs:9` | **No rate limits, and Argon2 is unbounded.** Login, vault unlock and recovery allow unlimited guesses. Concurrent calls can run about 512 Argon2 hashes at 19 MiB each. | Per-IP and per-account throttling, plus a semaphore around Argon2. |
| H7 | `crates/api/src/state/mod.rs:17,69`, `apps/web/src/hooks/prefs/usePrefs.ts:58` | **Vault keys never expire on the server.** Keys are per user. Session expiry and password change do not clear them, and auto-lock only runs in an open tab. Any session or `ldk_` key reads unlocked vaults with no passphrase. | Key by session, enforce the idle timeout on the server, and deny API keys vault access unless scoped. Related to the roadmap's per-session unlock. |
| H8 | `crates/db/src/repo/bookmarks.rs:876`, `activity.rs:67` | **Decrypted vault titles written to activity.** `summary` stores `renamed "old" → "new"` in plaintext, permanently, and survives `delete_vault`. | Log field names only for vault rows, and test for it. |
| H9 | `apps/web/src/services/url-display/urlDisplay.ts:9`, `BookmarkCardPreview.tsx:15`, `GlanceTile.tsx:31` | **Vault hostnames sent to Google.** Vault cards have no favicon, so `googleFavicon` runs on each render, sending the host and the viewer's IP. | Never call it for vault items. Preferably drop the third-party lookup. |
| H10 | `crates/api/src/routes/vault/mod.rs:266`, `routes/search/mod.rs:53` | **Vault move leaves plaintext in search.** Meilisearch cleanup is best-effort after commit, and search does not recheck Postgres, so a failed delete leaves the plaintext document returned while locked. | Outbox with retries, or recheck hits against `is_vault` and `deleted_at`. |
| H11 | `crates/api/src/routes/auth/mod.rs:168-211` | **OIDC has no state, nonce or PKCE.** Login CSRF signs a victim into the attacker's account, where they save bookmarks. | Short-lived cookie with state and PKCE, checked at callback. Validate the `id_token`. |
| H12 | `crates/api/src/routes/auth/mod.rs:285` | **OIDC ignores closed registration.** Any identity the provider accepts gets an account, and on an empty database becomes admin. | Apply the same gate as `register`. |
| H13 | `apps/web/src/routes/_app.tsx:7`, `apps/web/vite.config.ts:34`, `crates/api/src/routes/auth/mod.rs:187,236,264` | **OIDC sign-in does not work at all.** The guard reads a localStorage token OIDC never sets. The service worker returns `index.html` for `/api/*` navigations. No single issuer value builds all three Authentik URLs. | Guard on `/auth/me`, add `navigateFallbackDenylist: [/^\/api\//]`, and use discovery. |
| H14 | `crates/api/src/routes/auth/mod.rs:67-92`, `crates/db/src/repo/invites.rs:150,169` | **One registration invite creates many accounts.** Concurrent registers all pass the check, and the user row survives a failed consume. | Claim the invite and insert the user in one transaction. |
| H15 | `crates/api/src/metrics.rs:77-146`, `crates/api/src/routes/health/mod.rs:51` | **Unauthenticated metrics leak public-list slugs.** Raw paths are labels, so slugs are readable and random paths grow memory without limit. `/api/ready` also returns raw error text. | Label with `MatchedPath`, protect the route, and return booleans from ready. |
| H16 | `crates/db/src/repo/bookmarks.rs:214`, `shares.rs:407,440` | **Write invitee can take a bookmark.** `PATCH {category_ids:[own]}` removes all of the owner's links, including private folders. Their own category then gives permanent access after revoke or block. | Non-owners only change links in the owner's tree, and keep the others. Do not derive access from a foreign owner's category. |
| H17 | `crates/db/src/repo/dashboards.rs:233,403` | **Dashboard pins bypass shares.** Pins show notes with no read recheck, survive revoke and block, and `owner_pin` lets a write invitee pin any bookmark by id. | Recheck `bookmark_can_read` at render. Accept only ids already pinned. Remove foreign pins on revoke. |
| H18 | `crates/db/src/repo/categories.rs:944`, `moderation.rs:381` | **Public lists ignore `is_private`.** "Only me" bookmarks stay visible at `/l/{slug}`, and the moderation takedown relies on that flag. | Add `AND b.is_private = FALSE`, and limit to the category owner's bookmarks. |
| H19 | `crates/db/src/repo/shares.rs:41,407`, `bookmarks.rs:420` | **Invitees keep working in the owner's Trash.** Trashed folders and bookmarks stay readable and editable, and invitees with `show_deleted_in_library` list them. | Non-owners get NotFound for deleted rows or ancestors. |
| H20 | `crates/db/src/repo/dashboards.rs:371`, `apps/web/src/hooks/dashboards/useDashboardEdit.ts:12` | **Saving a layout deletes pins the editor cannot see.** Vault pins while locked, trashed pins and other owners' vault pins are lost for good. | Keep items hidden from the actor, or diff rather than replace. |
| H21 | `apps/web/src/hooks/library/useLibrarySelection.ts:36`, `pages/library/LibraryBulkBar.tsx:22` | **Select all in Explorer selects the whole library.** In a 3-item folder of 500, Delete or Tag → Replace hits all 500. | Select from the rendered folder, or hide bulk select there. |
| H22 | `crates/db/src/repo/categories.rs:465`, `trash.rs:124`, `migrations/001_init.sql:29` | **Purging a parent deletes live children.** Restoring only the child, or creating under a trashed parent, leaves a live folder that a later purge cascades away. | Reparent on restore, reject trashed parents, and reparent live children before purge. |
| H23 | `crates/core/src/import_export/mod.rs:26`, `crates/db/src/repo/import.rs:98` | **HTML export drops live bookmarks.** Bookmarks in trashed or shared folders are silently left out. | Treat unknown category ids as uncategorised, and emit under each reachable category. |
| H24 | `crates/db/src/repo/bookmarks.rs:1415,1438`, `apps/web/src/services/library/mergeDuplicates.ts:5` | **Duplicate merge can trash the only live copy.** Duplicate queries include trashed rows and may pick one as the keeper. | Add `deleted_at IS NULL` to both queries. |
| H25 | `crates/core/src/models/mod.rs:60,917`, `crates/db/src/repo/bookmarks.rs:926` | **Clearing a field does nothing.** JSON `null` becomes "unchanged", so folder to root, icon, description, bio, name and timezone all report success with no change. | Double-option deserializer. Make description nullable. |
| H26 | `deploy/docker/docker-compose.yml:33`, `docker-compose.dev.yml:37` | **Chrome 124 runs with `--no-sandbox`.** A renderer exploit gets a shell in the container. Production is now isolated from the database and search, but the dev stack is not. | Maintained Chrome image, sandbox where possible, memory and pid limits, and the same networks in dev. |

## Medium

| ID | Where | Defect | Suggested fix |
| --- | --- | --- | --- |
| M1 | `crates/db/src/repo/users.rs:453` | Password change keeps all sessions and API keys. | Delete other sessions, offer key revocation, and clear vault keys. |
| M2 | `crates/db/src/repo/users.rs:205` | Last-admin check is racy: two admins can demote each other, with no recovery short of SQL. | Check and update under the ban advisory lock. |
| M3 | `crates/api/src/routes/auth/mod.rs:54`, `users.rs:34` | Bootstrap admin is racy, and whoever registers first owns a fresh install. | Advisory lock, plus an optional `BOOTSTRAP_TOKEN`. |
| M4 | `apps/web/src/lib/auth/authToken.ts:8` | Session token in localStorage undoes `HttpOnly`. No XSS sink is known today. | Rely on the cookie in the browser. |
| M5 | `crates/core/src/crypto/mod.rs:155` | Argon2 at the OWASP floor with an 8-byte minimum, and parameters are not stored, so cost cannot be raised later. | Store parameters per wrap, raise to 64 MiB or more, and raise the minimum length. |
| M6 | `crates/api/src/routes/vault/mod.rs:161,206,228,239`, `vault_moves.rs:304` | Sensitive vault actions need no passphrase, some while locked: replacing the recovery key, changing the global passphrase, and purging locked items. | Require a passphrase or fresh unlock. Block purge and move on locked content. |
| M7 | `crates/db/src/repo/trash.rs:35`, `categories.rs:178,239` | Locked vault category names are served by Trash, activity and `GET /categories/{id}`. | Encrypt names, or mask them and return 423. |
| M8 | `crates/db/src/repo/vault_moves.rs:220` | Move-in leaves plaintext tags, comments, audit titles and category activity. | Delete unused tags, and extend the scrub. |
| M9 | `crates/db/src/repo/vault_moves.rs:343`, `migrations/026_profile_visibility.sql:14` | Moving a followers or members category into a vault returns 500. | Set `visibility='private'`, or add a blocker. |
| M10 | `crates/db/src/repo/categories.rs:280` | Move crosses vault and owner boundaries, so deleting the other tree cascades into this one. | Require same owner and same `vault_id`. |
| M11 | `crates/worker/src/lib.rs:74-123`, `crates/api/src/routes/bookmarks/mod.rs:256,270` | In-flight enrichment leaves assets for a row just moved to a vault, and `/favicon` and `/og` have no vault check. | Recheck after each write, and refuse vault rows on those routes. |
| M12 | `crates/db/src/repo/vault_moves.rs:210,264`, `bookmarks.rs:950` | Vault moves take no row lock, so a concurrent PATCH can write plaintext into a vault row. | `FOR UPDATE`, a version check, and `is_vault=FALSE` on plain updates. |
| M13 | `crates/db/src/repo/vault_moves.rs:169` | Dashboard blocker is not owner-scoped: someone else's stale pin blocks a move forever and leaks that dashboard's title. | Delete foreign pins, and block only on owned dashboards. |
| M14 | `crates/db/src/repo/categories.rs:603` | Fork copies trashed content and is not transactional. | Filter `deleted_at`, and use one transaction. |
| M15 | `crates/db/src/repo/dashboard_shares.rs:67` | Invitees can view and edit trashed dashboards. | NotFound for non-owners when deleted. |
| M16 | `crates/api/src/routes/bookmarks/mod.rs:114`, `crates/db/src/repo/bookmarks.rs:1213`, `routes/vault/mod.rs:273` | Trashed bookmarks return to search through any PATCH, boot backfill or vault move-out. | Guard `upsert_bookmark` on `deleted_at`, and filter the worker query. |
| M17 | `crates/db/src/repo/pending_shares.rs:489` | Owners cannot see or revoke a rejected invite, and the invitee can restore it at any time. | Let owners revoke, or make rejection final. |
| M18 | `crates/db/src/repo/bookmarks.rs:192` | Shared views show the owner's other folder names and paths. | Return only readable categories, with paths cut at the shared root. |
| M19 | `crates/db/src/repo/categories.rs:942`, `follows.rs:144`, `social.rs:147` | Email local part leaks to anonymous visitors and others. | Use a `profile_label` style fallback. |
| M20 | `crates/db/src/repo/social.rs:96` | Likes and comments fail below a share root, because only direct shares count. | Walk ancestors. |
| M21 | `crates/worker/src/lib.rs:52,299` | Response bodies are read fully before any size check, and titles are stored with no length limit. | Stream with a byte cap, and truncate. |
| M22 | `crates/api/src/routes/bookmarks/mod.rs:95,367`, `routes/import_export/mod.rs:85` | Enrichment concurrency is per call, so a burst opens hundreds of Chrome tabs. | One global semaphore or bounded queue. |
| M23 | `crates/core/src/avatar/mod.rs:28`, `preview_image/mod.rs:31` | No dimension limit on image decode: N uploads cost N × 512 MiB. | Read header dimensions first, and gate decodes. |
| M24 | `crates/api/src/main.rs:65` | Documented upload limits are unreachable: axum's 2 MiB default rejects larger avatars, previews and imports. | Per-route `DefaultBodyLimit`, after H3. |
| M25 | `crates/api/src/main.rs:27`, `crates/db/src/repo/trash.rs:89` | Retention only runs at startup and only removes rows: preview files, search documents and social rows stay. | Periodic task. Return ids and clean up. |
| M26 | `crates/core/src/import_export/mod.rs:177` | JSON import flattens nesting when a child sorts before its parent. | Topological sort. Error on an unresolved parent. |
| M27 | `crates/db/src/repo/import.rs:61`, `crates/core/src/import_export/mod.rs:413` | Path-keyed import turns `AC/DC` into `DC` and merges same-name siblings. | Key by node, not joined names. |
| M28 | `crates/api/src/routes/import_export/mod.rs:62` | Import is partial on failure: the job stays `running` and rows are missing from search. | Background job with error status, and batch reindex. |
| M29 | `crates/db/src/repo/bookmarks.rs:40,609,928` | Multi-step bookmark writes have no transaction. A failed PATCH half-applies, a NUL in a tag wipes all tags, and concurrent edits are lost. | One transaction with `FOR UPDATE`. Validate first. |
| M30 | `crates/db/src/repo/import.rs:125` | Exports are not a full backup: extra categories, flags, timestamps and dashboards are missing, and `<DD>` is not parsed. | Extend the JSON format and version it. |
| M31 | `crates/db/src/repo/bookmarks.rs:403,492`, `categories.rs:944` | Hot lists are unbounded with N+1 queries, including the anonymous public list. | Batch-load views, paginate, and cap public lists. |
| M32 | `deploy/docker/docker-compose.yml:9,22,50,54`, `crates/api/src/config/mod.rs:41`, `README.md:80` | Known default secrets in the production stack, and the README's root `.env` is likely ignored by compose. | `${VAR:?required}`, reject known defaults, and use `--env-file`. |
| M33 | `deploy/docker/Dockerfile:20`, `docker-compose.yml:5,19,45` | API runs as root, and `db`, `meilisearch` and `api` have no restart policy. | Unprivileged `USER`, and `restart: unless-stopped`. |
| M34 | `apps/web/src/api/client.ts:44`, `pages/app-shell/AppShell.tsx:93` | No 401 handling: an expired session sticks on "Failed to load account" and old data stays on screen. | Clear the token and cache, and go to `/login`. |
| M35 | `apps/web/src/hooks/auth/useAuth.ts:13,24,35` | Login does not clear the previous user's cache, tabs are not told about logout, and a failed logout reports success while the session stays valid. | `qc.clear()` on login, a `storage` listener, and report failure. |
| M36 | `apps/web/src/hooks/bookmarks/useBulkBookmarks.ts:59` | Bulk add overwrites from a client snapshot, and a missing id wipes tags and categories. | Server-side add, or fail on a miss. |
| M37 | `apps/web/src/services/dashboards/dashboardDraft.ts:49` | A section with a blank title is silently dropped with its pins. | Block save with a validation error. |
| M38 | `apps/web/src/hooks/dashboards/useDashboardPage.ts:23` | An edit draft can be saved onto a different dashboard after losing access. | Bind the draft to its dashboard id. |
| M39 | `apps/web/src/pages/library/LibraryPageModals.tsx:14`, `useLibrarySelection.ts:27` | The permanent-delete warning reads the filtered list, so an irreversible delete says it can be restored. Selection is never pruned. | Use the selected ids against the full set. Prune on change. |
| M40 | `apps/web/src/hooks/prefs/usePrefs.ts:76` | Failed auto-lock reports locked, and leaves decrypted data on screen. | Keep the unlocked state, retry, and always drop caches. |
| M41 | `apps/web/src/hooks/library/useLibraryUrlState.ts:28` | Search input loses a trailing space, joining the next word. | Skip the reset when `urlQ === q.trim()`. |
| M42 | `apps/web/src/pages/library/LibraryResults.tsx:17` | Search in Explorer on the Vaults page always shows nothing. | Pass `lib.list` as the search items. |
| M43 | `apps/web/src/hooks/library/useLibraryList.ts:111` | Grid view ignores `folder`, so a palette jump changes nothing and can disable Add. | Filter by folder and show a chip, or force Explorer. |
| M44 | `apps/web/src/hooks/library/useLibraryBookmarkEditor.ts:53` | The vault unlock modal reopens on every search, taking focus mid-typing. | Remember dismissal for the session. |
| M45 | `apps/web/src/components/avatar/AvatarCropModal.tsx:77`, `pages/settings/ModerationDrawer.tsx` | Esc closes the modal and the parent drawer together, losing drafts. | Use `closeOnEscape={!childOpen}` or stacks. |
| M46 | `apps/web/src/pages/library/LibraryBulkBar.tsx:52` | Bulk Move to vault is offered for shared items the API refuses, failing the whole batch. | Send owned ids only, or disable with a reason. |
| M47 | `apps/web/src/hooks/auth/useAuth.ts:49`, `pages/onboarding/OnboardingPage.tsx:80` | Finishing onboarding can bounce back, depending on timing. Not reproduced. | Update the cache from the response. |
| M48 | `apps/web/src/pages/settings/GeneralSettingsPanel.tsx:47` | Trash retention saves per keystroke, so typing 365 can store 3 and a later purge deletes items older than 3 days. | Draft locally and save on blur. |
| M49 | `apps/web/src/pages/register/RegisterPage.tsx:93` | The invite gate is replaced after the first typed character, losing focus. | Gate on `search.invite`. |
| M50 | `apps/web/src/components/bookmark/BulkEditDrawer.tsx:106`, `category/CategoryPublicLinkSection.tsx:102`, `vault/VaultList.tsx:106`, `social/SocialDrawer.tsx:135` | No confirmation on Replace tags (even when empty), Rotate public link, Replace recovery key and comment delete. | Confirm each, and require non-empty tags. |

## Low

| ID | Where | Defect | Suggested fix |
| --- | --- | --- | --- |
| L1 | `crates/db/src/repo/users.rs:493` | Login timing reveals which emails exist. | Verify against a dummy hash. |
| L2 | `crates/api/src/routes/auth/mod.rs:289` | A local account can block a victim's first OIDC login. | Email verification, or an approved link flow. |
| L3 | `crates/db/src/repo/sessions.rs:47` | Expired sessions are never purged. | Periodic purge. |
| L4 | `crates/api/src/routes/users/mod.rs:211`, `follows.rs:42`, `shares.rs:274` | Hidden users and blocks can be detected through counts and status codes. | Gate through `require_viewable_profile`, and use uniform responses. |
| L5 | `crates/db/src/repo/social.rs:405` | Authors cannot delete comments after losing access, and owners cannot delete comments on their items. | Author check only for deletion. |
| L6 | `crates/db/src/repo/shares.rs:155` | Nested shares show write where read is enforced. | Compute the nearest grant. |
| L7 | `crates/db/src/repo/bookmarks.rs:968` | A write invitee's tag edit moves tags out of the owner's namespace. | Use the bookmark owner's namespace. |
| L8 | `crates/core/src/models/mod.rs:755`, `crates/api/src/state/mod.rs:87`, `crates/core/src/crypto/mod.rs:154` | Keys are not zeroized, and `VaultKeyring` derives `Debug` over raw keys. | `Zeroizing`, redacted `Debug`, and `Arc` the ring. |
| L9 | `crates/core/src/crypto/mod.rs:99,169` | No AAD on ciphertext, and `public_key` is unauthenticated. Needs database write access. | AAD of vault and bookmark id. Verify the key on open. |
| L10 | `crates/db/src/repo/vault_moves.rs:66,206,261` | An invitee-created bookmark blocks a move with a bare 404, and `is_private` is lost on a round trip. | Blocker kind, and add the field to the payload. |
| L11 | `crates/db/src/repo/vault.rs:15`, `vault_moves.rs:46,395` | Trashed vault items appear in vault search, and move-out re-indexes trashed rows. | Filter `deleted_at`. |
| L12 | `crates/api/src/routes/bookmarks/mod.rs:231`, `crates/db/src/repo/bookmarks.rs:1217` | A deleted preview comes back at the next backfill, and permanent failures retry every boot. | Suppression flag or attempted-at marker. |
| L13 | `deploy/docker/docker-compose.dev.yml:13,30,71,97` | The dev stack publishes known-credential services on all interfaces, sharing volumes with production. | Bind to `127.0.0.1`. |
| L14 | `crates/worker/src/lib.rs:31,50,146,670` | Worker failures are silent, and "enriched" logs on failure. | `warn!` at each. |
| L15 | `crates/db/src/repo/bookmarks.rs` (`record_link_check`), `dashboards.rs` (`put_dashboard`), `tags.rs` | Link checks churn `updated_at`, `put_dashboard` uses the pool inside a transaction, and tag rows are never reaped. | Separate column, use the transaction, and reap. |
| L16 | `apps/web/src/pages/banned/BannedPage.tsx:27` | Spinner forever on a 5xx or network failure, with no Sign out. | Error and retry state. |
| L17 | `apps/web/src/services/bookmarks/bookmarkPreview.ts:9` | Public list previews return 401 for anonymous visitors. | Public asset route keyed by slug. |
| L18 | `apps/web/src/components/bookmark/BookmarkPreviewField.tsx:133` | After cancelling a crop, the same file cannot be picked again. | Add `resetRef`. |
| L19 | `apps/web/src/hooks/library/useLibraryPageChrome.ts:60` | Esc in any overlay wipes the selection. | Skip when an overlay is open. |
| L20 | `apps/web/src/pages/login/LoginPage.tsx:100`, `components/api-keys/ApiKeysPanel.tsx:97` | Held Enter submits repeatedly, creating several sessions or API keys. | Return early while pending. |
| L21 | `apps/web/src/hooks/avatar/useAvatarImage.ts:18`, `crates/api/src/routes/users/mod.rs:171` | The previous user's avatar is served from HTTP cache for up to an hour. | Use `/users/{id}/avatar`, or `no-store`. |
| L22 | `apps/web/src/hooks/library/useLibrarySelection.ts:13,79` | Bulk failures carry no detail, and selection clears on partial failure. | Keep failed ids selected, with the reason. |
| L23 | `apps/web/src/lib/library/librarySearch.ts:37` | Tags containing a comma cannot be filtered. | Repeat the param. |
| L24 | `crates/api/src/main.rs:76` | SPA deep links return a 404 status with the app's HTML. | Use `fallback` rather than `not_found_service`. |
| L25 | Repository root | No `.dockerignore`, so local `node_modules` can overwrite the image's. | Add one. |
| L26 | `apps/web/src/components/library-explorer/FolderTile.tsx:49` | A button is nested in a button. | Separate the elements. |
| L27 | `crates/db/src/repo/bookmarks.rs` (`create_bookmark`) | URLs have no scheme check. React 19 blocks `javascript:` links, and the worker refuses non-web URLs, so this is defence in depth. | Reject non-http(s) at create, update and import. |
| L28 | `apps/web/src/hooks/images/useApiImage.test.ts` | The test "loads API images with the page token" failed once under load and passed on rerun. | Check for a timing dependency. |

## Behaviour changes from the fixes

- URLs on private addresses, including `.local`, `.lan` and single-label names, no longer get metadata, previews or link checks. This affects homelab bookmarks. An allowlist setting could restore it if wanted.
- A person with no account now needs the invite link. Registering with the invited address without the link attaches nothing. If they do that, the owner can invite them again to reach their account.
- Invites created before migration 033 have no link. Invite again to get one.
- Cached favicons and OG images must be PNG, JPEG, GIF, WebP, ICO or SVG by content. Anything else falls back to the remote URL on the card.
