import { useMemo } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useMe } from '@/hooks/auth'
import { useCategoryFolderActions } from '@/hooks/categories'
import { buildLibrarySearch } from '@/lib'
import type { Category } from '@/schemas'
import { FOLDER_ALL, FOLDER_UNCATEGORIZED, type ExplorerFolderId } from '@/services'
import { useUi } from '@/stores'
import { useLibraryBookmarkEditor } from './useLibraryBookmarkEditor'
import { useLibraryList } from './useLibraryList'
import { useLibraryPageChrome } from './useLibraryPageChrome'
import { useLibrarySelection } from './useLibrarySelection'
import { useLibraryUrlState } from './useLibraryUrlState'
import { useLibraryViewMode } from './useLibraryViewMode'

export type { LibraryViewMode } from './types'

/** Compose library URL / list / selection / editor hooks for the Library page. */
export function useLibraryPage() {
  const nav = useNavigate()
  const me = useMe()
  const showHidden = useUi((s) => s.showHidden)

  const url = useLibraryUrlState()
  const { viewMode, onViewModeChange } = useLibraryViewMode()
  const listState = useLibraryList({
    debouncedQ: url.debouncedQ,
    filterCats: url.filterCats,
    filterTags: url.filterTags,
    filterFlags: url.filterFlags,
    filterSince: url.filterSince,
    vaultOnly: url.vaultOnly,
    sort: url.sort,
    folderId: url.folderId,
    filterActive: url.filterActive,
  })

  const selection = useLibrarySelection({
    list: listState.list,
    bookmarks: listState.bookmarks.data || [],
  })

  const folderCat = useMemo(() => {
    if (!url.folderId || url.folderId === FOLDER_ALL || url.folderId === FOLDER_UNCATEGORIZED) {
      return null
    }
    return (listState.cats.data || []).find((c) => c.id === url.folderId) ?? null
  }, [listState.cats.data, url.folderId])

  const editor = useLibraryBookmarkEditor({
    bookmarks: listState.bookmarks.data || [],
    searchHits: listState.search.data?.hits || [],
    folderCat,
    vaultPrompt: !!listState.search.data?.vault_prompt,
  })

  const chrome = useLibraryPageChrome({
    canCreate: editor.canCreateInContext,
    selectMode: selection.selectMode,
    openCreate: editor.openCreate,
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
        to: '/library',
        search: buildLibrarySearch({
          q: url.debouncedQ.trim() || undefined,
          sort: url.sort,
          category: url.filterCats,
          tag: url.filterTags,
          flag: url.filterFlags,
          since: url.filterSince,
          vault: url.vaultOnly,
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
      onDelete: () => categoryActions.openDelete(cat.id),
      onMoveToVault: () => categoryActions.openMove(cat),
      forkPending: categoryActions.forkPending,
    }
  }

  return {
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
