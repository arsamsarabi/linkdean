import { useMemo } from 'react'
import type { SortKey } from '@/lib'
import type { Bookmark, Category, FolderCounts } from '@/schemas'
import {
  filterAndSortLibraryItems,
  FOLDER_ALL,
  folderBreadcrumb,
  isSharedFolder,
  listFolderContents,
  sortExplorerFolders,
  type ExplorerFolderId,
  type FlagFilter,
  type LibraryListItem,
} from '@/services'

type Input = {
  categories: Category[]
  bookmarks: Bookmark[]
  searchHits: LibraryListItem[]
  folderId: ExplorerFolderId | undefined
  sort: SortKey
  showHidden: boolean
  filterFlags: FlagFilter[]
  folderCounts?: FolderCounts
}

/** Derive explorer folder/bookmark lists for the current folder + search hits. */
export function useLibraryExplorerContents({
  categories,
  bookmarks,
  searchHits,
  folderId,
  sort,
  showHidden,
  filterFlags,
  folderCounts,
}: Input) {
  const selectedId: ExplorerFolderId = folderId || FOLDER_ALL
  const atAll = selectedId === FOLDER_ALL

  const byId = useMemo(() => new Map(categories.map((c) => [c.id, c])), [categories])
  const crumbs = useMemo(() => folderBreadcrumb(selectedId, byId), [selectedId, byId])
  const contents = useMemo(
    () => listFolderContents(selectedId, categories, bookmarks, showHidden, folderCounts),
    [selectedId, categories, bookmarks, showHidden, folderCounts]
  )
  const sortedFolders = useMemo(() => sortExplorerFolders(contents.folders, sort), [contents.folders, sort])
  const ownedFolders = useMemo(
    () => (atAll ? sortedFolders.filter((f) => !isSharedFolder(f)) : sortedFolders),
    [atAll, sortedFolders]
  )
  const sharedFolders = useMemo(() => (atAll ? sortedFolders.filter(isSharedFolder) : []), [atAll, sortedFolders])
  const sortedBookmarks = useMemo(
    () =>
      filterAndSortLibraryItems({
        items: contents.bookmarks,
        filterCats: [],
        filterTags: [],
        filterFlags,
        sort,
      }),
    [contents.bookmarks, filterFlags, sort]
  )
  const sortedHits = useMemo(
    () =>
      filterAndSortLibraryItems({
        items: searchHits,
        filterCats: [],
        filterTags: [],
        filterFlags,
        sort,
      }),
    [searchHits, filterFlags, sort]
  )

  return {
    selectedId,
    atAll,
    byId,
    crumbs,
    sortedFolders,
    ownedFolders,
    sharedFolders,
    sortedBookmarks,
    sortedHits,
  }
}
