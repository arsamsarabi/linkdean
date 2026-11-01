# Vaults Page Design

## Goal

Give open vaults their own page that works like the Library, and remove the vault list from the user menu.

## Decisions

- The page reuses the Library code with a scope. It is not a copy, so sorting, filters, views and bulk actions stay the same.
- Whether unlocked vault items also appear in the Library is a user setting. It defaults to on, which matches today.
- Whether deleted vault items go to Trash or are removed immediately is a user setting. It defaults to Trash, which matches today.
- Vaults are unlocked from the Vault settings tab. The command palette has an "Unlock vault" command that goes there. The user menu has no vault entries.

## Route and navigation

- New route `/vaults`, rendering `LibraryPage` with `scope="vault"`.
- `/vault` keeps redirecting to `/settings?tab=vault`.
- A "Vaults" button sits after Library in the header, and in the mobile nav. It shows only while at least one vault is unlocked, using the `vaultUnlocked` store value that hydrates from `me.vault_unlocked`.
- The user menu loses the vault list and "Unlock all". The green dot on the avatar stays.

## The page

`useLibraryPage` takes a scope of `library` or `vault`.

With `scope="vault"`:

- Bookmarks and categories are limited to unlocked vault items. Locked vaults are not named or counted anywhere on the page.
- A vault filter sits with the other filters, as a URL param, listing unlocked vaults by name.
- The page header has a Lock button per unlocked vault and Lock all.
- Loose vault bookmarks appear under Unsorted in the explorer view.
- Search filters the decrypted list in the browser and makes no search request, because Meilisearch holds no vault content.
- The "Vault only" filter, the unlock-for-search prompt and the Library's URL state stay out of this page. It has its own URL state.
- Actions that vault items cannot use are hidden: share, public link, duplicates, likes and comments, save a copy.
- The bulk categories picker lists only categories in the same vault. Bookmarks whose vault differs from the picked category are reported as failed, like any other bulk failure.

With `scope="library"`, behaviour depends on the new setting below.

## Settings

Migration `031_vault_page_settings.sql` adds to `user_settings`:

- `show_vault_in_library BOOLEAN NOT NULL DEFAULT TRUE`
- `vault_delete_mode TEXT NOT NULL DEFAULT 'trash'`, checked to be `trash` or `permanent`

Both join `UserSettings` and `UpdateSettings`. Both appear in Settings → Vault.

### Show vault items in Library

When on, the Library behaves as it does today.

When off:

- The Library requests bookmarks and categories without vault items.
- The "Vault only" filter is hidden.
- The "Vault in search" setting is hidden and the unlock-for-search prompt never opens.
- `/search` treats `vault_search_mode` as `never`, so the API agrees with the page.
- Dashboards are unchanged. Vault pins are already blocked by the move rules.

### Deleted vault items

When `trash`, deleting works as it does today.

When `permanent`:

- `DELETE /bookmarks/{id}` and `DELETE /categories/{id}` call the existing purge functions for vault rows, on the server, so every client gets the same result.
- The confirm dialog, including bulk delete, says the delete cannot be undone.
- Deleting a vault category removes the category only. As with any category delete, its bookmarks lose that folder and stay in the vault.
- Vault items already in Trash stay there until retention removes them or the user purges them.

## Unlocking

- Settings → Vault stays the place to unlock each vault.
- The Global passphrase card gains an "Unlock all" button when the global passphrase is on, opening the existing `UnlockAllVaultsModal`.
- The command palette gains an "Unlock vault" item, shown while any vault is locked. It navigates to `/settings?tab=vault`.

## Edge cases

| Case | Behaviour |
| --- | --- |
| Last vault locks while on `/vaults` | Redirect to `/library` with a notification. Covers manual lock, auto-lock and another device. |
| `/vaults` opened with nothing unlocked, including after an API restart | Redirect once `me` has loaded, without rendering vault content first |
| Lock of any vault | Decrypted data is removed from the query cache, not only invalidated: `bookmarks`, `categories`, `search` and dashboard queries. Back shows nothing. |
| Another device locks a vault | The tab learns when `me` and the lists refetch, on window focus |
| One of several vaults locks | Its filter value and selected bookmarks are dropped |
| Creating with two or more vaults open | Vault is required, prefilled from the vault filter. With one open it is preselected. |
| Open vault is empty | The link stays. The empty state offers Add bookmark and a hint to use Move to vault from the Library. |
| Move out from the page | The item leaves the list, and the notification says it moved to the Library |
| Palette jump to a vault category | Goes to `/vaults` when the Library hides vault items, otherwise the Library |
| Vault bookmarks in Trash | Their stored title and URL are empty, so `list_trash` returns a blank name. It returns "Vault item" instead. |

## Testing

- DB: `permanent` removes the row and `trash` keeps it; `permanent` leaves non-vault items in Trash; settings round trip and reject an unknown mode.
- API: `/search` returns no vault hits when `show_vault_in_library` is off.
- Web: the Vaults link shows and hides with `vaultUnlocked`; the user menu has no vault entries; scope limits the list to vault items; the redirect fires when the last vault locks; lock removes cached queries; palette shows "Unlock vault" only while one is locked.
- Browser: with two vaults, walk the page, filter by vault, lock one, lock the last and land on the Library, press Back and see no vault content.

## Out of scope

- Inline unlock on the Vaults page or in the header
- Showing decrypted titles in Trash
- Per-vault delete settings
