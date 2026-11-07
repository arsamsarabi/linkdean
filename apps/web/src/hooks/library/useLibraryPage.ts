import { useMemo } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useMe } from '@/hooks/auth'
import { useCategoryFolderActions } from '@/hooks/categories'
import { buildLibrarySearch } from '@/lib'
import type { Category } from '@/schemas'
import { explorerSelectableItems, FOLDER_ALL, FOLDER_UNCATEGORIZED, type ExplorerFolderId } from '@/services'
import { useUi } from '@/stores'
import type { LibraryPath, LibraryScope } from './types'
import { useLibraryBookmarkEditor } from './useLibraryBookmarkEditor'
import { useLibraryList } from './useLibraryList'
import { useLibraryPageChrome } from './useLibraryPageChrome'
import { useLibrarySelection } from './useLibrarySelection'
import { useLibraryUrlState } from './useLibraryUrlState'
import { useLibraryViewMode } from './useLibraryViewMode'

export type { LibraryScope, LibraryViewMode } from './types'

/** Compose library URL / list / selection / editor hooks for the Library page. */
export function useLibraryPage(scope: LibraryScope = 'library') {
  const nav = useNavigate()
  const me = useMe()
  const showHidden = useUi((s) => s.showHidden)

  const basePath: LibraryPath = scope === 'vault' ? '/vaults' : '/library'
  const showVaultInLibrary = me.data?.settings.show_vault_in_library ?? true
  const vaultOnlyAvailable = scope === 'library' && showVaultInLibrary

  const url = useLibraryUrlState(basePath)
  const { viewMode, onViewModeChange } = useLibraryViewMode()
  const listState = useLibraryList({
    scope,
    vaultId: url.vaultId,
    showVaultInLibrary,
    debouncedQ: url.debouncedQ,
    filterCats: url.filterCats,
    filterTags: url.filterTags,
    filterFlags: url.filterFlags,
    filterSince: url.filterSince,
    vaultOnly: vaultOnlyAvailable && url.vaultOnly,
    sort: url.sort,
    folderId: url.folderId,
    filterActive: url.filterActive,
  })

  // Bulk actions must only reach what is on screen, and Explorer shows one folder at a time.
  const searchHits = listState.searchItems
  const selectable = useMemo(() => {
    if (viewMode !== 'explorer') return listState.list
    return explorerSelectableItems({
      searching: listState.searching,
      searchHits,
      folderId: url.folderId,
      categories: listState.cats.data || [],
      bookmarks: listState.bookmarks.data || [],
      showHidden,
      filterFlags: url.filterFlags,
      sort: url.sort,
    })
  }, [
    viewMode,
    listState.list,
    listState.searching,
    searchHits,
    url.folderId,
    listState.cats.data,
    listState.bookmarks.data,
    showHidden,
    url.filterFlags,
    url.sort,
  ])

  // Explorer at All lists folders, not bookmarks, so there is no match for Select all to reach.
  const atExplorerRoot = viewMode === 'explorer' && !url.folderId
  const selection = useLibrarySelection({
    list: selectable,
    matchFilter: atExplorerRoot ? null : listState.matchFilter,
  })

  const folderCat = useMemo(() => {
    if (!url.folderId || url.folderId === FOLDER_ALL || url.folderId === FOLDER_UNCATEGORIZED) {
      return null
    }
    return (listState.cats.data || []).find((c) => c.id === url.folderId) ?? null
  }, [listState.cats.data, url.folderId])

  const editor = useLibraryBookmarkEditor({
    bookmarks: listState.bookmarks.data || [],
    searchHits: listState.searchItems,
    folderCat,
    vaultPrompt: scope === 'library' && !!listState.search.data?.vault_prompt,
    scope,
    vaultId: url.vaultId,
  })

  const chrome = useLibraryPageChrome({
    basePath,
    vaultId: url.vaultId,
    canCreate: editor.canCreateInContext,
    selectMode: selection.selectMode,
    openCreate: editor.openCreate,
    openCreateWithUrl: editor.openCreateWithUrl,
    exitSelectMode: selection.exitSelectMode,
    searchParams: url.searchParams,
    urlQ: url.urlQ,
    sort: url.sort,
    filterCats: url.filterCats,
    filterTags: url.filterTags,
    filterFlags: url.filterFlags,
    filterSince: url.filterSince,
    vaultOnly: url.vaultOnly,
    folderId: url.folderId,
  })

  const categoryActions = useCategoryFolderActions({
    onDeleted: (id) => {
      if (url.folderId !== id) return
      void nav({
        to: basePath,
        search: buildLibrarySearch({
          q: url.debouncedQ.trim() || undefined,
          sort: url.sort,
          category: url.filterCats,
          tag: url.filterTags,
          flag: url.filterFlags,
          since: url.filterSince,
          vault: url.vaultOnly,
          vaultId: url.vaultId,
          folder: FOLDER_ALL,
        }),
      })
    },
  })

  function onFolderChange(id: ExplorerFolderId): void {
    url.setLibrarySearch({ folder: id })
  }

  function categoryMenuFor(cat: Category) {
    const socialEnabled = cat.shared_out || cat.access === 'read' || cat.access === 'write'
    return {
      onEdit: () => categoryActions.openEdit(cat),
      onShare: () => categoryActions.openShare(cat),
      onSocial: socialEnabled ? () => categoryActions.openSocial(cat) : undefined,
      onFork: () => categoryActions.forkCategory(cat),
      onDelete: () => categoryActions.openDelete(cat.id, cat.is_vault),
      onMoveToVault: () => categoryActions.openMove(cat),
      forkPending: categoryActions.forkPending,
    }
  }

  return {
    scope,
    vaultOnlyAvailable,
    me,
    showHidden,
    viewMode,
    onViewModeChange,
    ...url,
    ...listState,
    ...selection,
    ...editor,
    ...chrome,
    folderCat,
    categoryActions,
    onFolderChange,
    categoryMenuFor,
  }
}
