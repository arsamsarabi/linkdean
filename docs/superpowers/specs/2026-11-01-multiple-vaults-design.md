# Multiple vaults

## Goal

A user can create several named vaults, each with its own passphrase. A vault holds categories and loose bookmarks. Existing bookmarks and categories can move into, out of, and between vaults.

This replaces the roadmap item "Per-category vault passphrases".

## Decisions

| Topic | Decision |
| --- | --- |
| Vault model | Named vault holding many categories and loose bookmarks |
| Keys | Each vault has its own random data key, wrapped by its passphrase |
| Blocked moves | Refuse the move and list every blocker with instructions to resolve it |
| Lost passphrase | A recovery key sets a new passphrase. There is no email reset, because the server cannot unwrap a vault key |
| Deleting a vault | Allowed with contents after explicit confirmation |
| Global passphrase | Optional and off by default. Unlocks chosen vaults in one step and never replaces a vault's own passphrase |

## Data model

Migration `029_multiple_vaults.sql`:

- `vaults` gains `id UUID PRIMARY KEY DEFAULT gen_random_uuid()` and `name TEXT NOT NULL`. `user_id` stops being the primary key and stays `NOT NULL`.
- A unique index covers `(user_id, lower(name))`.
- `vaults` gains nullable `recovery_salt`, `recovery_wrapped_key` and `recovery_wrapped_nonce`.
- `vaults` gains nullable `global_wrapped_key` and `global_wrapped_nonce`.
- `categories` and `bookmarks` gain `vault_id UUID REFERENCES vaults(id)`.
- A new table `vault_master_keys` has `user_id` as primary key, plus `kdf_salt`, `wrapped_key` and `wrapped_nonce`.
- Each user's existing vault is named `Vault`, and their `is_vault` rows get its `vault_id`.
- `is_vault` stays, so existing checks keep working. A check constraint on each table enforces `is_vault = (vault_id IS NOT NULL)`.

Rules the repo layer enforces:

- A vault bookmark sits only in categories of the same vault, or in none (a loose bookmark).
- A vault category's parent is in the same vault, or it has no parent.
- Vault names and category names stay plaintext so the app can list what to unlock. Bookmark content stays encrypted.

## Keys

`AppState.vault_keys` becomes a map from `(user_id, vault_id)` to the data key. One vault can be open while another stays locked. Keys stay in memory only, so an API restart locks every vault.

`bookmark_to_view` and the search, list and trash paths look up the key for the bookmark's own vault. Lists and search show only unlocked vaults. Vault search scans each unlocked vault.

Auto-lock after inactivity locks every unlocked vault.

## Recovery key

- Creating a vault returns a recovery key once: 32 random bytes shown as grouped base32. The UI requires the user to confirm they saved it before continuing.
- The key wraps a second copy of the data key, stored in the `recovery_*` columns. Only that wrapped copy is stored.
- `POST /vaults/{id}/recover` takes the recovery key and a new passphrase, rewraps the data key under the new passphrase, and unlocks the vault.
- A vault that is unlocked can create or replace its recovery key from Settings. This covers vaults that existed before this feature. Replacing one invalidates the previous key.
- Losing both the passphrase and the recovery key loses the contents. The creation form says so.

## Global passphrase

A convenience so a user need not type several passphrases. It adds a way in. Each vault keeps its own passphrase, which goes on working.

- Enabling it in Settings sets a global passphrase. The server creates a random master key wrapped by that passphrase and stores it in `vault_master_keys`.
- A user chooses which vaults join. Joining needs the vault unlocked and the global passphrase, because the vault's data key gets wrapped by the master key into `global_wrapped_*`. New vaults do not join automatically.
- A vault can leave at any time, which clears its `global_wrapped_*` columns.
- Unlock all takes the global passphrase and unlocks every joined vault. Vaults that have not joined stay locked.
- Changing the global passphrase needs the current one and rewraps only the master key.
- Disabling it deletes the master key row and clears every vault's `global_wrapped_*` columns. No vault contents or vault passphrases change.
- A forgotten global passphrase costs nothing: disable it, then enable it again. Disabling needs no passphrase for that reason.
- Recovery and passphrase changes on a vault do not affect its membership, because the data key stays the same.
- Settings warns that anyone who learns the global passphrase can open every joined vault.

## API

| Route | Purpose |
| --- | --- |
| `GET /vaults` | Each vault's id, name, unlocked state, whether it has a recovery key, whether it joined the global passphrase, and item counts for unlocked vaults only |
| `POST /vaults` | Create with name and passphrase; returns the recovery key and leaves the vault unlocked |
| `PATCH /vaults/{id}` | Rename |
| `DELETE /vaults/{id}` | Delete with contents after confirmation |
| `POST /vaults/{id}/unlock` | Unlock with passphrase |
| `POST /vaults/{id}/lock` | Lock |
| `POST /vaults/lock-all` | Lock every vault, used by auto-lock |
| `POST /vaults/{id}/recover` | Set a new passphrase with the recovery key |
| `POST /vaults/{id}/recovery-key` | Create or replace the recovery key; the vault must be unlocked |
| `POST /vaults/move` | Move categories and bookmarks; see below |
| `GET /vaults/global` | Whether the global passphrase is enabled |
| `POST /vaults/global` | Enable with a new global passphrase |
| `PUT /vaults/global` | Change it with the current and new passphrase |
| `DELETE /vaults/global` | Disable it |
| `POST /vaults/global/unlock` | Unlock every joined vault; returns the ids unlocked |
| `POST /vaults/{id}/global` | Join, with the global passphrase; the vault must be unlocked |
| `DELETE /vaults/{id}/global` | Leave |

`/vault/setup`, `/vault/unlock`, `/vault/lock` and `/vault/status` are removed. There are no external API users before the first public release.

Bookmark and category create requests take an optional `vault_id` for loose bookmarks. A category chosen inside a vault still implies that vault.

`DELETE /vaults/{id}` takes the typed vault name and the account password. Users who sign in with SSO and have no password give the name only. It permanently removes the vault's categories and bookmarks, including trashed ones.

## Moving items

`POST /vaults/move` takes `category_ids`, `bookmark_ids` and `destination_vault_id`, where null means out of any vault.

Moving a category also moves its subcategories and its bookmarks, including trashed ones. A moved category whose parent is not moving loses its parent.

Every vault involved must be unlocked, or the route returns the existing vault-locked error.

### Blockers

The route checks everything before changing anything. If any blocker exists it returns `409` with all of them, and nothing moves. Blockers apply to moves into a vault, from outside or from another vault.

| Blocker | Instruction shown |
| --- | --- |
| Category shared with people, or has open invites | Remove N people from sharing on "Reading" |
| Category has a public link | Turn off the public link on "Reading" |
| Bookmark also sits in a category that is not moving | Remove "Rust docs" from "Work" |
| Bookmark pinned on a dashboard | Remove "Rust docs" from dashboard "Home" |

Each blocker carries a kind, the subject ids and the display text, so the dialog can link to the place that fixes it.

### Moving in

One transaction per request:

1. Encrypt `url`, `title`, `description`, `notes` and tags into `vault_ciphertext`.
2. Clear those columns plus `favicon_url`, `preview_image_url` and `has_preview`, and delete the tag rows.
3. Clear `is_private`, which has no meaning in a vault.
4. Replace `subject_title` with a generic label in `activity_events` and `notifications` for the moved subjects, and clear any stored link-check result.
5. Set `is_vault` and `vault_id`.

After commit, remove the bookmarks from Meilisearch and delete their preview, favicon and Open Graph files. A failure here is logged; the existing repair paths reindex search, and the files are unreachable because asset routes exclude vault bookmarks.

### Moving out or between

- Out: decrypt, restore the plain columns and tags, clear the vault columns, index in Meilisearch, and queue enrichment.
- Between vaults: decrypt with the source key and encrypt with the destination key.

## Web app

- Settings: `VaultPanel` becomes a vault list with create, rename, unlock, lock, recovery key and delete. Forgotten passphrase leads to the recovery form.
- Creation shows the recovery key once, with copy and a required "I saved this" confirmation.
- Settings: a global passphrase section enables, changes and disables it, and each vault has a switch to join or leave.
- Header: the lock menu lists each vault and its state, plus "Unlock all" when a global passphrase is enabled.
- Library: "Move to vault" appears on the bookmark menu, on bulk select and on the category menu. The dialog picks a destination, or "Out of vault".
- Blocked moves show a checklist of every blocker, each linking to the place that resolves it.
- Unlock prompts name the vault.
- Delete asks for the vault name and account password, and states that contents cannot be recovered.

## Errors

| Case | Result |
| --- | --- |
| Wrong passphrase or recovery key | The existing unauthorized-style error |
| Duplicate vault name | `409` |
| Vault locked during a move or edit | The existing vault-locked error |
| Another user's vault | `404` |
| Wrong name or password on delete | `400`, nothing deleted |

## Out of scope

- Changing a passphrase with the current passphrase. Recovery covers the lost case.
- Email reset.
- Per-session unlock. Unlocking on one device still unlocks that vault for all of the user's sessions. Added to the roadmap.
- Rate limiting on vault unlock and recovery. Argon2id slows guesses, but nothing caps attempts today. Belongs to the API hardening idea.
- Client-held keys (E2E). A separate project.

## Testing

- DB suite: vault isolation with two vaults, one locked; migration of an existing vault; every blocker; no partial move when blocked; plain columns, tags and subject titles cleared on move in; round trip back out; move between vaults; recovery with the right and wrong key; replaced recovery keys stop working; delete with contents; global unlock opens only joined vaults; own passphrases keep working alongside it; leaving, changing and disabling the global passphrase; another user's vault returns not found.
- Route smoke test on a throwaway database.
- Browser run: create two vaults, move items in, hit a blocker, resolve it, lock one vault, recover a passphrase.
