# Read-only MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a read-only Streamable HTTP MCP endpoint at `/api/mcp` so agents with an API key can search and read the library (no vault, no writes).

**Architecture:** Mount official `rmcp` Streamable HTTP on the Axum API behind `EXPERIMENTAL_AGENT_SKILLS`. Auth reuses Bearer/`AuthUser` logic via middleware that inserts `User` into request extensions; tools read `Parts` and call existing `db` / search helpers. Folder browse includes descendants (same recursive tree as Agent Skills).

**Tech Stack:** Rust (`linkdean-api`, `linkdean-db`, `linkdean-search`), `rmcp` (Streamable HTTP server + macros), Axum 0.8, React/Mantine web UI.

**Spec:** [docs/design/mcp-readonly.md](../../design/mcp-readonly.md)

## Global Constraints

- Path `/api/mcp`; tools only (no MCP resources); no write tools.
- Gate with `EXPERIMENTAL_AGENT_SKILLS` (same as agent-skill zip route).
- Prefer stateless Streamable HTTP (`NeverSessionManager` / `legacy_session_mode(false)` per current `rmcp` docs).
- Vault folders/bookmarks never in tool results; trash never included; shared “Only me” notes follow REST/skill rules.
- `search_bookmarks`: empty `q` + folder/tag → SQL browse; non-empty `q` → Meili then filters; folder = that folder **+ descendants**.
- Crate boundaries: thin API MCP module; SQL/visibility in `db`; no duplicated authz.
- Pin latest `rmcp` that compiles against workspace Axum 0.8 at implement time; features `server`, `macros`, `transport-streamable-http-server` (and session feature if required by that version).
- Commits on a feature branch from `origin/main` (or stacked on design PR if still open); push each green task per linkdean-repo-workflow.
- After code changes: `graphify update .` from repo root.

## File map

| Path | Role |
| --- | --- |
| `Cargo.toml` (workspace) | Pin `rmcp` |
| `crates/api/Cargo.toml` | Depend on `rmcp` with server features |
| `crates/api/src/mcp/mod.rs` | Router nest, auth middleware, `StreamableHttpService` wiring |
| `crates/api/src/mcp/server.rs` | `ServerHandler` / `#[tool]` impls |
| `crates/api/src/mcp/tools.rs` | Argument structs + call orchestration → db/search |
| `crates/api/src/mcp/dto.rs` | JSON shapes returned to agents (bookmark summary, folder, tag, archive status) |
| `crates/api/src/routes/mod.rs` | Merge MCP router when `agent_skills` |
| `crates/db/src/repo/mcp_browse.rs` (or extend `agent_skills.rs`) | Folder-tree browse + filter helpers (descendant IDs, list for empty-`q`) |
| `crates/db/src/repo/mod.rs` | Re-export |
| `apps/web/src/components/import-export/McpConnectCard.tsx` | Leading MCP UI block |
| `apps/web/src/components/import-export/McpConnectCard.test.tsx` | Flag on/off, URL, key link |
| `apps/web/src/components/import-export/AgentSkillCard.tsx` | Secondary copy |
| `apps/web/src/components/import-export/ImportExportPanel.tsx` | Render MCP card above Agent Skill |
| `apps/web/src/components/import-export/index.ts` | Barrel |
| `docs/guides/agent-skills.md` | MCP connect section |
| `docs/product/features.md`, `docs/CHANGELOG.md`, `docs/product/roadmap.md` | Ship wording / Phase A done |
| `docs/design/agent-skills.md` | Already points at mcp-readonly; update if needed |
| `.env.example` / self-host env docs | Mention MCP under the same experimental flag |

## Review Focus

Failure modes the happy-path tests can miss — each has a pinned test in the owning task:

1. Flag off → `/api/mcp` unavailable (404), not an empty MCP server.
2. Missing/invalid Bearer → HTTP 401/unauthorized before tool dispatch.
3. Vault folder id in `search_bookmarks` / vault bookmark in `get_bookmark` → tool error, no plaintext.
4. Shared bookmark with `is_private` owner notes → notes omitted for non-owner (match skill/REST).
5. `get_archive_status` never returns file bytes or reader HTML — metadata fields only.

---

### Task 1: Mount authenticated MCP scaffold behind the flag

**Files:**
- Modify: `Cargo.toml`, `crates/api/Cargo.toml`
- Create: `crates/api/src/mcp/mod.rs`, `crates/api/src/mcp/server.rs`
- Modify: `crates/api/src/lib.rs` or `main` module tree / `crates/api/src/routes/mod.rs` (merge when `agent_skills`)
- Test: `crates/api/src/mcp/mod.rs` (`#[cfg(test)]`) or `crates/api/tests/mcp_http.rs` if the crate already uses integration tests that pattern

**Interfaces:**
- Consumes: `AppState`, `cfg.experimental_agent_skills`, `auth::extract_token` / `db::user_from_token` / ban checks (same rules as `AuthUser`)
- Produces: `pub fn router() -> Router<AppState>` nesting `StreamableHttpService` at `/mcp` (final path `/api/mcp` via existing `.nest("/api", …)`); inserts authenticated `User` into request extensions for tools

- [ ] **Step 1: Write failing tests**

```rust
#[tokio::test]
async fn mcp_is_absent_when_experimental_flag_is_off() { /* GET or POST /api/mcp → 404 */ }

#[tokio::test]
async fn mcp_rejects_unauthenticated() { /* flag on, no Bearer → 401 */ }

#[tokio::test]
async fn mcp_initialize_ok_with_api_key() { /* flag on + ldk_ key → MCP initialize succeeds; tools/list may be empty until Task 3 */ }
```

Use the existing API test harness / `AppState` builder patterns already used for agent-skill or auth tests. If none fit HTTP MCP easily, a tower `oneshot` against `mcp::router().with_state(state)` is enough.

- [ ] **Step 2: Run tests — expect FAIL** (router/module missing)

- [ ] **Step 3: Implement scaffold**

- Add workspace `rmcp` pin; enable required features on `linkdean-api`.
- `StreamableHttpService::new` with server factory; config: **stateless** (`NeverSessionManager` and/or `legacy_session_mode(false)` — follow pinned `rmcp` docs).
- Axum layer: resolve user like `AuthUser`; on failure return HTTP error; on success `req.extensions_mut().insert(user)`.
- Merge only when `agent_skills` is true in `api_router` (same boolean as agent_skills zip).
- Empty tool list OK for this task.

- [ ] **Step 4: Run tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/api/Cargo.toml crates/api/src/mcp crates/api/src/routes/mod.rs
git commit -m "$(cat <<'EOF'
Add experimental /api/mcp Streamable HTTP scaffold with API-key auth.
EOF
)"
```

---

### Task 2: DB browse helper for folder tree (+ tag) without Meili

**Files:**
- Create: `crates/db/src/repo/mcp_browse.rs`
- Modify: `crates/db/src/repo/mod.rs`
- Test: `crates/db/tests/mcp_browse.rs` (or `#[cfg(test)]` in-module if small)

**Interfaces:**
- Consumes: `require_category_read`, recursive tree pattern from `agent_skill_bookmarks`
- Produces:
  - `pub async fn mcp_folder_descendant_ids(pool, user_id, folder_id) -> Result<Vec<Uuid>>` — read-checked root; vault/deleted excluded; descendants non-vault
  - `pub async fn mcp_browse_bookmarks(pool, user_id, folder: Option<Uuid>, tag: Option<&str>, limit: i64) -> Result<Vec<BookmarkView>>` — empty-`q` path; folder includes descendants; tag = own bookmarks only (same as skills); `include_vault: false`; clamp limit 1..=100 default 50
  - Reject vault root folder with `BadRequest` / clear message (match skill)

- [ ] **Step 1: Write failing tests** (need `DATABASE_URL` throwaway DB)

```rust
#[tokio::test]
async fn browse_folder_includes_bookmarks_in_subfolders() { /* assert child folder bookmark appears */ }

#[tokio::test]
async fn browse_rejects_vault_folder() { /* … */ }

#[tokio::test]
async fn browse_tag_is_own_bookmarks_only() { /* … */ }
```

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement `mcp_browse.rs` + re-export**

Reuse the `WITH RECURSIVE tree` approach from `agent_skills.rs`; return views via existing `bookmarks_to_views` / plain paths — **no vault keyring unlock**.

- [ ] **Step 4: Run — expect PASS** (`DATABASE_URL=postgres://linkdean:linkdean@localhost:55432/linkdean_test cargo test -p linkdean-db mcp_browse -j 2`)

- [ ] **Step 5: Commit**

```bash
git add crates/db/src/repo/mcp_browse.rs crates/db/src/repo/mod.rs crates/db/tests/mcp_browse.rs
git commit -m "$(cat <<'EOF'
Add MCP folder-tree browse helpers matching Agent Skill scope.
EOF
)"
```

---

### Task 3: Tools `search_bookmarks` and `get_bookmark`

**Files:**
- Create/modify: `crates/api/src/mcp/dto.rs`, `tools.rs`, `server.rs`
- Test: extend MCP HTTP/unit tests in `crates/api`

**Interfaces:**
- Consumes: Task 1 extensions `User` + `AppState`; Task 2 browse; `state.search.search`; `db::plain_bookmark_views_by_ids`, `db::bookmark_can_read`, `db::get_bookmark` / view helpers used by REST get (no vault unlock)
- Produces tools:
  - `search_bookmarks(q: Option<String>, folder: Option<Uuid>, tag: Option<String>, limit: Option<u32>) -> CallToolResult`
  - `get_bookmark(id: Uuid) -> CallToolResult`
  - DTO: id, title, url, notes (visibility-safe), tags, folder ids/names, created/updated — **never** vault ciphertext fields

Logic:
- Trim `q`; if empty → `mcp_browse_bookmarks` (require at least one of folder/tag, else `BadRequest`-style tool error: “give q, folder, and/or tag”).
- If non-empty `q` → Meili like `routes/search/mod.rs` (readable category ids with `list_categories(..., unlocked_vaults: &[], …)` so vaults stay out of Meili filter set as today for locked); then filter hits to folder descendants and/or tag if set; cap `limit` 1..=100.
- `get_bookmark`: not found / no read → tool error; vault bookmark → tool error (“vault not available via MCP”); never return vault body.

- [ ] **Step 1: Write failing tests**

```rust
#[tokio::test]
async fn search_with_q_returns_hits() { /* … */ }

#[tokio::test]
async fn search_empty_q_browses_folder_tree() { /* … */ }

#[tokio::test]
async fn get_bookmark_hides_vault() { /* … */ }

#[tokio::test]
async fn get_bookmark_omits_others_only_me_notes() { /* if fixtureable; else assert via db helper unit */ }
```

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement tools + register on server**

Map `linkdean_core::Error` → `rmcp::ErrorData` with plain `message` only.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/mcp
git commit -m "$(cat <<'EOF'
Add MCP search_bookmarks and get_bookmark tools.
EOF
)"
```

---

### Task 4: Tools `list_folders`, `list_tags`, `get_archive_status`

**Files:**
- Modify: `crates/api/src/mcp/server.rs`, `tools.rs`, `dto.rs`
- Test: `crates/api` MCP tests

**Interfaces:**
- Consumes: `db::list_categories(pool, user_id, include_hidden, &[], false)` then **filter out `is_vault`**; `db::list_tags`; `db::bookmark_can_read` + `db::list_archives`
- Produces:
  - `list_folders()` → id, name, parent_id, path-friendly fields agents need (no vault rows)
  - `list_tags()` → name (+ count if Tag already has it)
  - `get_archive_status(bookmark_id)` → list of `{ format, status, bytes?, updated_at? }` from `BookmarkArchiveView` — **no** file contents, no download URLs that stream bodies (omit or leave link-only if view has URL fields — prefer status metadata only)

- [ ] **Step 1: Write failing tests**

```rust
#[tokio::test]
async fn list_folders_excludes_vault() { /* … */ }

#[tokio::test]
async fn get_archive_status_is_metadata_only() { /* assert JSON has format/status; no html body key */ }

#[tokio::test]
async fn get_archive_status_requires_read() { /* other user’s bookmark → error */ }
```

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement remaining tools**

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/mcp
git commit -m "$(cat <<'EOF'
Add MCP list_folders, list_tags, and archive status tools.
EOF
)"
```

---

### Task 5: Web UI — MCP lead, zip secondary

**Files:**
- Create: `apps/web/src/components/import-export/McpConnectCard.tsx`, `McpConnectCard.test.tsx`
- Modify: `AgentSkillCard.tsx` (shorter secondary framing), `ImportExportPanel.tsx`, barrel `index.ts`
- Optional one-liner on API keys settings panel if a natural spot exists (no new key type)

**Interfaces:**
- Consumes: `useExperimentalFlags().agent_skills`, `window.location.origin` + `/api/mcp`
- Produces: Card with MCP URL, “create an API key” link to settings tab, short capability list (search/read; no vault), example header snippet

- [ ] **Step 1: Write failing UI tests** (Vitest + existing AgentSkillCard patterns)

```tsx
it('shows MCP card when experimental agent skills are on', () => { /* … */ })
it('hides MCP card when flag off', () => { /* … */ })
it('shows /api/mcp URL', () => { /* … */ })
```

- [ ] **Step 2: Run — expect FAIL** (`pnpm exec vitest run` on the test file from `apps/web`)

- [ ] **Step 3: Implement card; render above Agent Skill in Import/Export; soften Agent Skill title/copy to secondary**

Exact user-facing copy (use these strings unless a later polish PR changes them):

- Title: `MCP`
- Body lead: `Connect an agent with an API key for live search and read of your library. Vault items are never included.`
- Secondary Agent Skill lead: keep zip instructions but open with `Or download a static Agent Skill (.zip) for offline use.`

- [ ] **Step 4: Run tests + `pnpm exec prettier --check` on touched files + `pnpm exec tsc -b`**

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/import-export
git commit -m "$(cat <<'EOF'
Lead Import/Export with MCP connect; keep Agent Skill zip secondary.
EOF
)"
```

---

### Task 6: Docs, features, changelog, roadmap Phase A

**Files:**
- Modify: `docs/guides/agent-skills.md`, `docs/product/features.md`, `docs/CHANGELOG.md` under `## [Unreleased]`, `docs/product/roadmap.md` (rewrite/remove Phase A bullet; leave B/C), `docs/self-hosting/environment.md` (or equivalent experimental section), `docs/design/agent-skills.md` if stale
- Infra public guide/roadmap mirror in a paired PR when product docs ship (same change set per roadmap skill)

**Interfaces:**
- Consumes: shipped behaviour from Tasks 1–5
- Produces: connect example (Bearer header); experimental/hosted-off wording preserved

- [ ] **Step 1: Update guide with MCP section + client config example from the design**

- [ ] **Step 2: Features bullet + changelog line; roadmap Phase A marked done/removed**

- [ ] **Step 3: Grep docs for stale “Agent Skill only” agent-access claims and fix**

- [ ] **Step 4: Commit**

```bash
git add docs
git commit -m "$(cat <<'EOF'
Document read-only MCP and mark Agent Skills Phase A shipped.
EOF
)"
```

- [ ] **Step 5: `graphify update .`**; open/update PR; paired infra docs PR if public pages changed

---

## Self-review (plan author)

| Spec requirement | Task |
| --- | --- |
| `/api/mcp` + rmcp Streamable HTTP | 1 |
| API key auth / flag | 1 |
| Stateless preference | 1 |
| search empty-q browse / Meili+filters / folder+descendants | 2–3 |
| get_bookmark + vault exclusion | 3 |
| list_folders / list_tags / archive metadata | 4 |
| UI MCP lead, zip secondary | 5 |
| Docs / features / changelog / roadmap | 6 |
| No writes / no archive body / no resources | Global + Review Focus tests |

Review Focus items mapped to Tasks 1, 1, 3, 3, 4 respectively.
