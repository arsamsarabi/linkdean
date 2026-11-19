# Read-only MCP server (Agent Skills Phase A)

**Status:** approved for planning (product conversation 2026-11-19).  
**Roadmap:** Agent Skills, next steps → Phase A — MCP. Write tools are a later item (MCP write tools), after API key permissions.

Related: [agent-skills.md](./agent-skills.md) (zip / `SKILL.md` packaging), [guides/agent-skills.md](../guides/agent-skills.md).

## Intent

Let people point an MCP client at their Linkdean instance with an API key so agents can **search and read** the library live. This is for agent tooling users, not a personal daily driver. Zip Agent Skills remain for offline / static packs; MCP is the live path.

## Goals

- Remote **Streamable HTTP** MCP on the Linkdean API (official Rust SDK `rmcp`), same deployable as the app.
- **Read-only** tools: search, get bookmark, list folders, list tags, archive **metadata** only.
- Same privacy stance as Agent Skills: no vault, no trash, respect share visibility; no other people’s “Only me” notes.
- Auth via existing `Bearer ldk_…` API keys (session cookie OK for manual testing).
- Gated by `EXPERIMENTAL_AGENT_SKILLS` (same switch as zip skills).
- UI/docs **lead with MCP**; zip stays as an explicit secondary (“Download Agent Skill”).

## Non-goals (this phase)

- Write / mutate tools (roadmap: API key permissions, then MCP write tools).
- Archive HTML/PDF/reader body over MCP (Phase B — Archive text).
- Local stdio MCP binary.
- OAuth for MCP; a second auth system beside API keys.
- MCP resources (URI-style); tools only in v1.
- Turning the flag on for hosted beta (stays off until deliberately enabled).

## Anti-goals

- Agents reaching vault plaintext via MCP.
- Shipping a hand-rolled JSON-RPC that drifts from the MCP spec.
- Replacing or removing the zip Agent Skill API.

## Constraints

- Crate boundaries: thin API route/handler; reuse `db` / search / existing visibility rules — do not duplicate authz.
- Keys remain all-or-nothing until API key permissions ship; MCP still only **registers** read tools.
- Multi-instance self-host: prefer **stateless** Streamable HTTP (no sticky session requirement).
- Follow current MCP Streamable HTTP + `rmcp` docs for the crate version pinned at implement time.

## Approaches considered

1. **Mount `rmcp` Streamable HTTP in Axum** (chosen) — tools call the same core/db paths as REST. Spec-correct, one binary, fits existing API key auth.
2. Hand-roll a thin `/mcp` JSON-RPC — fewer deps, but easy to drift from the spec and break clients.
3. Separate MCP process that HTTP-calls REST — clean isolation, worse for self-host (extra process, latency, double config).

Local stdio was deferred as a later convenience, not Phase A.

## Design

### Endpoint

- Path: **`/api/mcp`**.
- Transport: Streamable HTTP via `rmcp` (`StreamableHttpService` or equivalent for the pinned version), nested on the Axum router.
- When `EXPERIMENTAL_AGENT_SKILLS` is false: endpoint unavailable (same idea as agent-skill route — 404 / not merged).
- Prefer stateless mode so load-balanced instances do not need session affinity.

### Auth

- Require `Authorization: Bearer <api key>` using the same extractor / validation as other API-key routes.
- Session cookie may work for browser/manual tests; docs and UI copy lead with API key.
- Banned users and disabled API keys fail like other key-authenticated routes.
- No vault unlock via API key; vault content must not appear in tool results.

### Tools (v1)

| Tool | Behaviour |
| --- | --- |
| `search_bookmarks` | Optional text `q` plus optional folder/tag filters; bounded page size. **Empty `q` with folder and/or tag:** library-list path (browse that scope). **Non-empty `q`:** Meili search, then apply folder/tag filters if set. **Folder filter includes subfolders** (same as Agent Skill zips), not “this folder only”. |
| `get_bookmark` | One bookmark: title, URL, notes, tags, folders, dates — same visibility as REST get. |
| `list_folders` | Folder tree the user can see, including shared; **exclude vault folders**. |
| `list_tags` | Tags for that library. |
| `get_archive_status` | Per-bookmark archive metadata (formats present, status). **Not** copy bodies. |

Always excluded: vault content, trash, writes, archive file bytes / reader text.

Tool errors: plain MCP errors (`not found`, vault/unavailable messages). No stack traces or internal paths.

### Implementation sketch (left to builder)

- New module under `crates/api` (e.g. `routes/mcp/` or `mcp/`) that owns the `rmcp` server types and tool handlers.
- Handlers call existing `db` / search helpers used by `routes/search`, bookmarks, categories, tags, archives list — do not reimplement visibility.
- Wire flag + OpenAPI/docs as appropriate (MCP may sit outside OpenAPI; document in the user guide).
- Pin `rmcp` (and features) in workspace `Cargo.toml`; verify Axum version compatibility.

### UI

When the experimental flag is on:

- **Config → Import / Export:** lead with an **MCP** block — URL (`https://<host>/api/mcp`), “use an API key”, short capability list, link to create a key.
- Agent Skill zip download stays on the same panel and on folder **⋯**, secondary.
- API keys settings: one line that keys work for MCP and agent-skill fetch (no new key type).

### Docs

- Extend [guides/agent-skills.md](../guides/agent-skills.md) (or a short sibling) with connect steps and a client config example.
- On ship: `features.md`, `[Unreleased]` changelog; public docs mirror via infra as usual.
- Keep “experimental / off on hosted beta” until the flag is turned on there.

Illustrative client config (field names follow client docs at ship time):

```json
{
  "mcpServers": {
    "linkdean": {
      "url": "https://your-linkdean/api/mcp",
      "headers": {
        "Authorization": "Bearer ldk_…"
      }
    }
  }
}
```

### Testing

- API / integration tests: flag off → unavailable; unauthenticated → rejected; search + get happy path; vault folder / vault bookmark excluded; shared vs only-me visibility matches REST.
- UI: unit tests for the MCP block when flag on/off (match AgentSkillCard patterns).
- Playwright optional for Phase A unless the UI block warrants a smoke stub later.

## Builder calls (settled here)

- Endpoint path `/api/mcp`, not a separate host.
- Tools-only (no MCP resources) in v1.
- Stateless Streamable HTTP preferred.
- Same experimental env flag as zip skills.
- `search_bookmarks`: empty `q` + folder/tag → library browse; non-empty `q` → Meili then filters; folder scope includes descendants.
- Exact `rmcp` version and Axum nest details: pick current stable that matches workspace Axum at implement time.

## Out of scope pointers

| Later work | Roadmap |
| --- | --- |
| Per-key read/write scopes + create templates | API key permissions |
| MCP create/update tools when scopes allow | MCP write tools |
| Archive reader text in agent surface | Agent Skills Phase B |
| Saved-search skills | Agent Skills Phase C |
