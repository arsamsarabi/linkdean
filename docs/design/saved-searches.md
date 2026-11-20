# Saved library searches (+ Agent Skills Phase C)

**Status:** Phase A (saved searches) and Phase C (Agent Skills from a saved search) shipped.  
**Roadmap:** Agent Skills Phase C done; MCP write tools shipped.

## Intent

Let people save the current library filter snapshot (query + filters) by name, reopen it from the library or the sidebar, and later turn that saved search into an Agent Skill.

## Goals (PR A — saved searches)

- Persist named filter snapshots per user in Postgres.
- CRUD over REST; enforce with existing `library:read` / `library:write` scopes.
- Reopen = navigate to `/library` or `/vaults` with the same URL params the library already understands.
- UI: Save + manage in library chrome; list in a sidebar section.

## Non-goals (PR A)

- Agent Skill from a saved search (Phase C).
- Sharing saved searches, drag reorder, folders of searches.
- Storing view mode (grid/list/explorer) or ephemeral URL keys (`create`, `focus`, `addUrl`).

## Filter snapshot

JSON fields (omit empty):

| Field | Notes |
| --- | --- |
| `q` | search string |
| `sort` | same keys as library URL |
| `category` | category id strings |
| `tag` | tag names |
| `flag` | `unread` / `favorite` / `pinned` |
| `since` | same keys as library |
| `vault` | bool |
| `vault_id` | uuid string |
| `folder` | folder id or library folder sentinel |

Reopening runs against **current** bookmark data (not a frozen id list).

## Data model

`saved_searches`: `id`, `owner_user_id`, `name` (1–80), `filters` JSONB, `created_at`, `updated_at`.  
Unique `(owner_user_id, lower(name))`. Soft cap 50 per user.

## API

- `GET /api/saved-searches`
- `POST /api/saved-searches` `{ name, filters }`
- `PATCH /api/saved-searches/:id` `{ name?, filters? }`
- `DELETE /api/saved-searches/:id`

Mapped to library scopes. Migration `062_saved_searches.sql`.

## UI

- Library toolbar: Save search when `q` or filters active; name modal; menu to open/rename/delete.
- Sidebar: collapsible Saved searches list → open; ⋯ rename/delete.

## Approaches considered

1. **`saved_searches` table + REST** (chosen).
2. Reuse dashboards — wrong model.
3. localStorage / URL-only — blocks Phase C.

## Phase C — Agent Skills from a saved search

### API

- Extend `GET /api/agent-skill` with `?search=<saved_search_id>` (mutually exclusive with `folder` / `tag`). Same `format=zip|md`.
- Still gated by `EXPERIMENTAL_AGENT_SKILLS` and `import_export:read`.
- Missing / not-owned search → `404`. Zero non-vault matches → `404` (same as tag skills).

### Live query

- Skill contents = bookmarks matching that search **now** (not a frozen id list).
- **Vault always excluded** from the skill payload: ignore `vault` / `vault_id` on the snapshot; never include vault bookmarks.
- Soft cap: same as folder/tag skills (`MAX_SKILL_BOOKMARKS` = 2000); skill notes how many were left out.
- Filters without `q`: map to the library page filter (folder, tags, flags, since presets resolved server-side, category OR). Own + readable shared non-vault bookmarks, same visibility rules as folder skills.
- Filters with `q`: Meilisearch for the query (limit at least the skill cap), then intersect with the same non-vault filters. Ranking follows Meili order among survivors.
- `SkillSourceKind::SavedSearch`; skill label = saved search name.

### UI

- **Config options → Import / Export → Agent Skill:** third source **Saved search** (select from the user’s list).
- **Saved search menus** (library toolbar Saved menu + sidebar ⋯): **Download as Agent Skill** when the experimental flag is on.
- Folder / tag skill entry points stay as they are.

### Non-goals (Phase C)

- Freezing bookmark ids at download time.
- Including vault bookmarks.
- Changing MCP tools (write tools remain a later roadmap item).

## Testing

### PR A (shipped)

- Repo: create/list/patch/delete; uniqueness; cap; owner isolation.
- API: session + scoped key; 403 without library write on mutate.
- Web: save from toolbar; sidebar open applies filters; delete confirm.

### Phase C

- Repo: saved-search skill includes matching non-vault bookmarks; vault filters ignored; empty → NotFound; owner isolation; cap truncation.
- Core: `SavedSearch` description / source line wording.
- API: `?search=` alone ok; combine with folder/tag → 400; OpenAPI regenerated.
- Web: Agent Skill card third segment; download from saved-search menu when flag on.
