# Saved library searches (+ Agent Skills Phase C)

**Status:** design approved; PR A implements saved searches only.  
**Roadmap:** unblocks Agent Skills — Phase C (skills from a saved search).

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

## Phase C — Agent Skills from a saved search (design only)

- Extend `GET /api/agent-skill` with `?search=<saved_search_id>` (alongside folder/tag).
- Skill contents = bookmarks matching that search **now** (live query).
- **Vault always excluded** from the skill payload, even if the saved search includes vault filters.
- UI: Import / Export and folder/tag skill entry points gain “Saved search”.
- Still gated by `EXPERIMENTAL_AGENT_SKILLS` and `import_export:read`.

## Testing (PR A)

- Repo: create/list/patch/delete; uniqueness; cap; owner isolation.
- API: session + scoped key; 403 without library write on mutate.
- Web: save from toolbar; sidebar open applies filters; delete confirm.
