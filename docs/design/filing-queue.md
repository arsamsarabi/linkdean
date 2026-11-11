# Filing queue for Unsorted

Design agreed with the product owner on 2026-11-11 for roadmap item 3 (Now). Fast capture makes saving one tap, so links pile up in Unsorted with no folder, few tags and sometimes no title. The filing queue lets a person work through that pile one bookmark at a time.

## Decisions

| Question | Decision |
| --- | --- |
| When is a bookmark filed? | **When it is in at least one folder.** Tags alone leave it in Unsorted, which matches what the Unsorted view already shows. |
| Bulk filing | **Not in the queue.** The library list already selects many bookmarks and moves them to a folder. The queue stays one card at a time. |
| Suggestions | **Folders and tags already used on the same site** (same host, ignoring `www.`). No page fetch per card, so no keyword suggestions and no rate-limit cost. |

## What the person sees

- **Entry points:** a **File** button in the Library header while Unsorted is open, and a count badge on the sidebar's Unsorted entry. The page lives at `/file`.
- **One card at a time,** newest first: preview, host and URL, editable title, description.
- **Per card:**
  - Folders: the bookmark form's folder field, including **Create "name"**.
  - Tags: the bookmark form's tag input.
  - A favorite toggle.
- **Actions:**
  - **File:** saves and moves on. Disabled until a folder is chosen.
  - **Skip:** leaves the bookmark in Unsorted and moves on. A skipped bookmark is not shown again until the page is reloaded.
  - **Delete:** moves the bookmark to Deleted items and moves on.
- **Suggestions:** up to 5 folders and 8 tags from the person's other bookmarks on the same site, most used first. A suggested folder is one click, or its number key.
- **Keys on desktop:** 1 to 5 add or remove a suggested folder. Enter files, S skips, D deletes. Keys are ignored while typing in a field, except Enter in the title field, which files.
- **Swipe on phones:** swipe left to skip, swipe right to file. Swiping right does nothing until a folder is chosen.
- **Progress and end:** a progress line ("12 left"). An empty state when nothing is left.

## API

- `GET /api/bookmarks/{id}/filing-suggestions` returns `{ folders: [{ id, count }], tags: [{ name, count }] }`.
  - It reads the owner's other bookmarks on the same host, leaving out deleted and vault ones.
  - Folders are the owner's own live, non-vault folders. The web app resolves their names and paths from the folder list it already has.
  - Only the owner can call it. Vault bookmarks get empty lists.
- The queue uses the existing endpoints:
  - `GET /api/bookmarks?folder=uncategorized&sort=created_desc` to list, and its `total` for the count.
  - `PATCH /api/bookmarks/{id}` to file.
  - `DELETE /api/bookmarks/{id}` to delete.

## Out of scope

- Bulk filing (see above).
- Keyword suggestions from a page fetch.
- Undoing a filed bookmark from the queue. The bookmark can still be edited from its folder.
