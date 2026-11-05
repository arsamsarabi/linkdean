# Library list pagination

Approved from roadmap **#4 Page the library list** (“go for it” / AFK). One PR.

## Goals

- Stop loading every bookmark into the SPA for library browse.
- Infinite scroll with **keyset** cursors (not `OFFSET`).
- Server-side folder / tag / flag / since / sort filters; return **total** match count.
- Explorer loads **one folder’s** contents (not a slice of the full library).
- “Select all” bulk actions apply to the **server-side match**, not only loaded rows.
- Vault search remains a Postgres scan of unlocked rows; page it the same way.
- Batch shared-bookmark access checks where they are still one-query-per-row.

## API

`GET /api/bookmarks` gains query params (all optional except auth):

| Param | Meaning |
| --- | --- |
| `limit` | Page size (default 50, max 100) |
| `cursor` | Opaque keyset cursor from the previous page |
| `sort` | Same keys the SPA uses today (`updated_at`, `created_at`, `title`, …) |
| `folder` | Category id, `uncategorized`, or omit for all |
| `tag` | Tag name/id filter |
| `flag` | favorite / unread / pinned (as today) |
| `since` | ISO timestamp lower bound on the active sort/time field |
| `q` | Optional text filter when not using Meili (vault path) |

Response:

```json
{ "items": [ /* BookmarkView */ ], "next_cursor": "…"|null, "total": 1234 }
```

Cursor encodes `(sort_value, id)` for the active sort; decode fails → `400`.

Unpaginated callers (export, reindex, admin) keep dedicated list helpers or `limit` omitted / high internal caps — do not break them.

## Web

- `useLibraryList` → infinite query (`useInfiniteQuery`) feeding the library grid/list.
- Drop client-side `filterAndSortLibraryItems` for the main library path; keep pure helpers only if tests still need them for offline logic.
- Explorer: request with `folder=<id>`; load more on scroll inside the folder view.
- Select-all: store “match selected” + current filter snapshot; bulk endpoints accept the same filter object (or resolve IDs server-side in one call).

## Bulk

Prefer: `POST /bookmarks/bulk` (or extend existing bulk routes) with `{ action, filters | ids }`. When `select_all` / filters present, run the action on the matched set under the same ACL as list. Cap destructive bulk size if needed; report how many were affected.

## Out of scope

- Numbered page UI
- Changing Meili search UX (search can stay its own path; library browse is the focus)
- Rewriting dashboards / public lists
