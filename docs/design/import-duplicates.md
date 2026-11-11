# Duplicates on import

Design agreed with the product owner on 2026-11-11 for roadmap item 17. Import adds every bookmark it reads, even when the library already has that URL, so importing the same file twice doubles the library. This design lets the person decide what happens to each duplicate before anything is written, and stops re-imports from duplicating folders.

## Decisions

| Question | Decision |
| --- | --- |
| What counts as the same bookmark | The **exact URL**, as in Library health → Duplicates and quick save. `https://a.com` and `https://a.com/` differ. Smarter matching, if it comes, changes all three together. |
| Which bookmarks are checked | The person's own live bookmarks **outside vaults**. Import never writes into a vault, and vault URLs are stored encrypted. A URL that is only in a vault imports as a normal bookmark. |
| What **Replace** does | **Updates the existing bookmark in place.** It keeps its ID and everything attached to it. |
| Folders on re-import | A folder whose **full path** matches an existing folder is **reused**. Only missing folders are created. |
| Where the state lives | Nowhere. The file is sent twice, once to preview and once to import. Nothing is staged on the server. |

## The three choices

For each imported bookmark whose URL is already in the library:

- **Skip**: the imported bookmark is dropped. The default.
- **Replace**: the existing bookmark takes the file's values (see below).
- **Merge**: the existing bookmark keeps its values, takes the file's title or description only where its own is empty, and gains the file's tags and folders (see below).
- **Import anyway**: a second copy is created, as every import does today.

## API

### `POST /api/import/preview`

Body: `{ format, content }`, the same as `POST /api/import`.

Parses the file with the same parsers and limits as import, writes nothing, and returns:

```json
{
  "total": 340,
  "new": 328,
  "duplicates": [
    {
      "index": 17,
      "url": "https://example.com/post",
      "existing": {
        "id": "…",
        "title": "…",
        "description": "…",
        "tags": ["…"],
        "folders": ["Work / Rust"],
        "updated_at": "…"
      },
      "incoming": {
        "title": "…",
        "description": "…",
        "tags": ["…"],
        "folders": ["Work / Rust"]
      }
    }
  ],
  "folders": { "reused": 12, "created": 3 }
}
```

- `index` is the bookmark's position in the parsed file. Parsing is deterministic, so the same file gives the same indexes on the import call.
- `folders` are full paths joined with ` / `, for display only.
- When the library holds several bookmarks at that URL, `existing` is the most recently updated one, the one **Replace** would update. This is the rule quick save already uses (`find_own_bookmark_by_url`).
- Limits: the same import byte limit (`effective_import_max`) and one-import-at-a-time slot as import. Its own rate limit (`import-preview:{user}`, 30 per 15 minutes), so previewing does not use up the 10 imports per 15 minutes.
- Not counted as an import job and not recorded in `import_jobs`.

### `POST /api/import`

Body gains an optional `duplicates` field:

```json
{
  "format": "netscape",
  "content": "…",
  "duplicates": {
    "default": "skip",
    "overrides": { "17": "replace", "42": "keep" }
  }
}
```

- Actions: `skip`, `replace`, `merge`, `keep` (Import anyway).
- No `duplicates` field means `keep` for every row: today's behavior, so onboarding and API clients do not change.
- Duplicates are found again when the import is applied, not taken from the preview. A bookmark that became a duplicate after the preview gets `default`. One that stopped being a duplicate is created. An override for an index that is not a duplicate is ignored.
- The bookmark quota check (`refuse_bookmark_quota`) counts only the bookmarks that will be created: new ones plus `keep` duplicates.
- The response gains `skipped`, `replaced` and `merged` beside `total`, `imported` and `failed`. `imported` counts bookmarks created. The `import_jobs` row keeps its current columns, so no migration is needed.

## Merge

Merge never removes anything from the existing bookmark:

- **Title** and **description** stay; the file's fill one only when the bookmark's is empty.
- **Tags** and **folders** from the file are added to the bookmark's own.
- Like Replace, it keeps the bookmark's ID, shares, pins and activity, and a failed merge counts as `failed`.

## Replace

The existing bookmark takes from the file only the values the file has:

- **Title** and **description**, when the file's value is not empty.
- **Tags**, when the file has any. They replace the existing tags.
- **Folders**, when the file puts the bookmark in at least one. They replace the existing folders. A bookmark with no folder in the file keeps its folders and is not moved to Unsorted.

Everything else stays as it is: ID, URL, shares, pins, dashboard sections, likes, ratings, comments, activity, preview image, favorite, unread, pinned, private, and created date. Replacing is an edit, so it goes through `update_bookmark`, touches `updated_at`, and is reindexed in search like any other edit. It does not queue a new screenshot.

## Folders

Before creating folders, the import maps each folder in the file to its full path from the root (`Work / Rust`) and looks for an existing folder with that exact path among the person's own live folders outside vaults. Names match exactly, case included.

- A match is reused, and bookmarks filed there go into the existing folder.
- No match: the folder is created under its parent, which may itself be reused.
- Two sibling folders in the file with the same name map to the same folder.
- Applies to every import, with or without duplicate decisions.

This is what keeps **Skip** from leaving a set of empty duplicate folders behind.

## Web app

In Settings → Import (`ImportExportPanel`):

1. **Import** reads the file and calls the preview first.
2. No duplicates: it goes straight on to the import, as today.
3. Duplicates: a **Review duplicates** drawer opens.
   - Header: "12 of 340 bookmarks are already in your library."
   - An apply-to-all choice: **Skip** (selected), **Replace**, **Merge**, **Import anyway**. Changing it sets every row that has not been changed by hand.
   - One row per duplicate: the URL, then the existing bookmark and the imported one side by side (title, description, tags, folders), with its own Skip / Replace / Merge / Import anyway choice.
   - Rows are paged 50 at a time, so a file with hundreds of duplicates stays usable.
   - **Import** sends the file with the decisions. **Cancel** imports nothing.
4. The result message adds the skipped and replaced counts, for example "Imported 328 of 340, replaced 4, skipped 8".

Onboarding keeps calling import directly with no decisions. A new account has nothing to duplicate, and folder reuse still applies.

## Errors

- The preview and import calls return the same errors as import today: unknown format, file too large, parse failure, rate limited, import already running, quota exceeded.
- A replace that fails counts as `failed` and the import goes on, as a failed create does today.
- If the import call fails, the drawer stays open with the error, so the person can retry without choosing again.

## Tests

Rust (database tests, run with `DATABASE_URL` set):

- Preview lists exact-URL duplicates with the right index, existing and incoming values, and writes nothing.
- Vault bookmarks and trashed bookmarks are not duplicates.
- Each action: `skip` creates nothing, `replace` updates in place and keeps ID, shares and pins, `merge` adds tags and folders and fills empty fields, `keep` creates a copy.
- Replace leaves the title, tags and folders alone when the file has none.
- No `duplicates` field behaves as `keep`.
- The quota counts only bookmarks that will be created.
- A bookmark that becomes a duplicate between preview and import gets `default`.
- Folder reuse by full path, including a reused parent with a new child, and same-name siblings.

Web (Vitest):

- No duplicates: import runs without opening the drawer.
- Apply-to-all sets untouched rows, and rows changed by hand keep their choice.
- The import call carries `default` and only the rows that differ from it.
- Cancel sends nothing.

## Out of scope

- URL normalization (trailing slash, `www.`, tracking parameters).
- Finding duplicates within the same file.
- Duplicates against vault bookmarks.
- Merging, as opposed to replacing, tags or folders.
