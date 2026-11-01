# Vaults Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `/vaults` page that reuses the Library, shown from a header link while a vault is open, plus the two new vault settings.

**Architecture:** `useLibraryPage` gains a `library` or `vault` scope. The scope picks the route, narrows bookmarks and categories, and turns search into a browser filter. Delete mode is enforced in the repo layer.

**Tech Stack:** Rust, axum, sqlx, Postgres 16, React, Mantine, TanStack Query and Router, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-01-vaults-page-design.md`

---

Work on `main`, commit and push each green task, and end commits with the `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` line. Run DB tests against the throwaway `linkdean-pg-check` container on port 55433. Run `graphify update .` after each commit.

### Task 1: Settings, delete mode, search, Trash label

**Files:** Create `migrations/031_vault_page_settings.sql`, `crates/db/tests/vault_settings.rs`. Modify `crates/core/src/models/mod.rs`, `crates/db/src/repo/{users,bookmarks,categories,trash}.rs`, `crates/api/src/routes/{bookmarks,categories,search}/mod.rs`.

- [ ] Write `crates/db/tests/vault_settings.rs`: settings round trip and reject an unknown mode; `permanent` removes a vault bookmark and a vault category with its subcategories; `trash` keeps them; `permanent` leaves plain bookmarks in Trash; a trashed vault bookmark is listed as "Vault item".
- [ ] Add the migration:

```sql
ALTER TABLE user_settings
    ADD COLUMN show_vault_in_library BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN vault_delete_mode TEXT NOT NULL DEFAULT 'trash'
        CHECK (vault_delete_mode IN ('trash', 'permanent'));
```

- [ ] Add both fields to `UserSettings` and `UpdateSettings`, and to `update_user_settings` with a validation match on the mode.
- [ ] Add `delete_bookmark_by_mode` and `delete_category_by_mode`. Each reads `is_vault` and `deleted_at` first, runs the existing delete, then purges only when the row was a live vault row and the mode is `permanent`. The two routes call these.
- [ ] In `/search`, use mode `never` when `show_vault_in_library` is off.
- [ ] In `list_trash`, return `Vault item` when `is_vault`.
- [ ] Run the Rust gate and commit: `Add the vault page settings and delete mode.`

### Task 2: Lock clears decrypted data

**Files:** Modify `apps/web/src/hooks/vault/useVault.ts`. Test `apps/web/src/hooks/vault/useVault.test.tsx`.

- [ ] Write a test with a real `QueryClient`: seed `['bookmarks', true]`, run `useLockVault`, and assert the cached data is gone.
- [ ] Add `resetDecrypted(qc)`, calling `resetQueries` on `bookmarks`, `categories`, `search` and `dashboard`. `resetQueries` drops data and refetches active observers, which `removeQueries` does not. Use it for lock, lock all and delete vault.
- [ ] Run the web gate and commit: `Clear decrypted data from the cache on lock.`

### Task 3: Library scope and the Vaults page

**Files:** Create `apps/web/src/routes/_app/vaults.tsx`, `apps/web/src/pages/library/VaultsPage.tsx`, `apps/web/src/pages/library/VaultsBar.tsx`, `apps/web/src/services/library/vaultScope.ts` with test. Modify `hooks/library/{useLibraryPage,useLibraryUrlState,useLibraryList,useLibraryPageChrome,useLibraryBookmarkEditor,types}.ts`, `lib/library/librarySearch.ts`, `pages/library/{LibraryPage,LibraryPageContext,LibraryPageHeader,LibraryFilterPanel,LibraryGridView,LibraryPageModals}.tsx`, `components/bookmark/BookmarkDrawer.tsx`, `components/category/CategoryDrawer.tsx`, `components/vault/MoveToVaultModal.tsx`.

- [ ] Add `vaultScope.ts`: `scopeBookmarks(items, vaultId)`, `scopeCategories(cats, vaultId)`, `matchesVaultQuery(item, q)`, with tests.
- [ ] Pass `scope` from `LibraryPage` through the provider to `useLibraryPage`, and derive `basePath` (`/library` or `/vaults`) for every `nav` call.
- [ ] Add the `vaultId` URL param to `LibrarySearch`, `validateLibrarySearch` and `buildLibrarySearch`.
- [ ] In `useLibraryList`, with `vault` scope, request vault items, narrow them, disable the search query and filter in the browser. With `library` scope and the setting off, request no vault items and ignore `vaultOnly`.
- [ ] Add `VaultsBar`: vault filter, Lock per open vault, Lock all. Clear `vaultId` when that vault closes.
- [ ] In `VaultsPage`, redirect to `/library` with a notification when `me` says nothing is unlocked.
- [ ] Prefill and require the vault when creating from this page. Use `Moved to Library` as the move-out message. Use the vault empty state. Show permanent-delete wording in confirm dialogs.
- [ ] Run the web gate and commit: `Add the Vaults page.`

### Task 4: Header, palette, settings UI, close out

**Files:** Create `components/vault/VaultPreferencesCard.tsx`. Modify `pages/app-shell/{AppShell,AppShellHeader}.tsx`, `services/command-palette/commandPalette.ts` and its test, `components/command-palette/CommandPalette.tsx`, `components/vault/{GlobalPassphraseCard,VaultPanel}.tsx`, `pages/settings/GeneralSettingsPanel.tsx`, `schemas/auth/auth.ts`, `docs/FEATURES.md`.

- [ ] Add the Vaults link to the header and mobile nav, and remove vault entries from the user menu.
- [ ] Add the palette `Unlock vault` item, and route vault category jumps to `/vaults` when the Library hides vault items.
- [ ] Add `VaultPreferencesCard` with both settings, an Unlock all button on the global card, and hide "Vault in search" when the Library hides vault items.
- [ ] Run a browser check with two vaults, update `docs/FEATURES.md`, run all gates and commit: `Add the Vaults link and vault page preferences.`
