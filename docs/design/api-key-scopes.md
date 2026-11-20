# API key permissions (scopes + templates)

**Status:** Phase A shipped; Phase B (templates) in this doc.  
**Roadmap:** API key permissions.

## Intent

API keys today act as the full user on REST (and MCP). Split **read vs write** by **function group** so a script or agent gets only what it needs.

## Goals

- Per-key scopes stored on `api_keys`, chosen at create time.
- Enforce on every `AuthUser` route when the caller is an API key (sessions stay unrestricted).
- Sensible groups: library, archive, import/export, social, account — each with `:read` and `:write`.
- Existing keys keep full access (migration default = all scopes).
- UI: checkboxes when creating a key; list shows granted scopes.
- MCP tools check the matching scopes (library/archive read).

## Non-goals

- Editing scopes on an existing key (revoke + create).
- OAuth / third-party apps.
- Scopes finer than the groups below (e.g. per-folder).
- Create API accepting a template id (client expands to `scopes`).

## Scope catalogue

| Scope | Covers (API key) |
| --- | --- |
| `library:read` | Bookmarks, folders, tags, search, dashboards, trash list, activity |
| `library:write` | Create/update/delete in those areas; open/check-link side effects |
| `archive:read` | List/get archive copies and Markdown; MCP `get_archive_*` |
| `archive:write` | Request/delete archives |
| `import_export:read` | Export; Agent Skill zip/md fetch |
| `import_export:write` | Import |
| `social:read` | Social summary; share invite inbox reads |
| `social:write` | Likes, ratings, comments; share invite actions |
| `account:read` | Profile/settings reads; notifications list |
| `account:write` | Profile/settings writes; vault unlock/ops; reports |

`/api/mcp` entry requires `library:read` **or** `archive:read`. Individual tools require the scope they need.

Exempt for keys: `/api/auth/*` (except key minting already `SessionUser`), health, public, metrics, OpenAPI. Unmapped authenticated paths: **fail closed** (403).

## Approaches considered

1. **Named scopes + path map in AuthUser** (chosen) — one enforcement point; handlers stay mostly unchanged.
2. Annotate every handler — precise but huge churn.
3. Only `read` / `write` umbrellas — too coarse vs roadmap.

## Design notes

- `CreateApiKey.scopes`: omit or null → all scopes (extension + scripts); empty array → 400.
- Unknown scope strings → 400.
- Browser extension minted keys get all scopes (same as omit).
- `ApiKeyView` includes `scopes` for the settings UI.
- Forbidden when missing: HTTP 403; MCP tool errors name the missing scope.

## Testing (Phase A)

- Create with subset; allowed route 200; other resource 403.
- Session cookie ignores scopes.
- Migration: pre-existing key has all scopes.
- MCP: library-only key can search; archive text needs `archive:read`.

---

## Phase B — Create-time templates

### Intent

Presets when creating a key so people do not tick every box. Enforcement and the create API stay Phase A (`scopes: string[]`).

### Catalogue

| Id | Label | Scopes |
| --- | --- | --- |
| `read_only_library` | Read-only library | `library:read` |
| `save_bookmarks` | Save bookmarks | `library:read`, `library:write` |
| `full_access` | Full access | all scopes |
| `custom` | Custom | whatever the checkboxes say |

Shared definitions in `crates/core/src/api_scopes.rs`. Web mirrors id/label/scope arrays (same sync comment as `ALL_SCOPES`).

### UI

- Segmented control above the permission checkboxes: the three presets + Custom.
- Default on open / after create: Full access (all scopes), matching Phase A default.
- Choosing a preset fills the checkboxes; checkboxes stay editable.
- Editing checkboxes: if the set matches a named preset, select that preset; otherwise Custom.
- Create still POSTs `scopes` only (no `template` field).

### Approaches considered

1. **Segmented control + shared core catalogue** (chosen) — least UI churn; one source of truth for scope sets.
2. Use-case cards with long copy — heavier for Settings.
3. API `template` field expanded server-side — unnecessary; create already takes scopes.

### Testing (Phase B)

- Core: each template’s scopes parse; `matching_template` returns the right id / `None` for custom sets.
- UI: pick preset → checkboxes update; tweak a box → Custom; create still works.
