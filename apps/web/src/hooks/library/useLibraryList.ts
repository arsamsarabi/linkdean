import { useMemo } from 'react'
import { useBookmarks, useSearchBookmarks } from '@/hooks/bookmarks'
import { useCategories } from '@/hooks/categories'
import { categoryPath, type SortKey } from '@/lib'
import {
  collectTagOptions,
  filterAndSortLibraryItems,
  FOLDER_UNCATEGORIZED,
  mergeSearchWithLocalCatchUp,
  sinceFilterToIso,
  type ExplorerFolderId,
  type FlagFilter,
  type LibraryListItem,
  type SinceFilter,
} from '@/services'

type Input = {
  debouncedQ: string
  filterCats: string[]
  filterTags: string[]
  filterFlags: FlagFilter[]
  filterSince: SinceFilter | null
  vaultOnly: boolean
  sort: SortKey
  /** `undefined` = All folders (`normalizeFolderId`). */
  folderId: ExplorerFolderId | undefined
  filterActive: boolean
}

/** Bookmarks + search merge, filtered list, and label maps. */
export function useLibraryList({
  debouncedQ,
  filterCats,
  filterTags,
  filterFlags,
  filterSince,
  vaultOnly,
  sort,
  folderId,
  filterActive,
}: Input) {
  const cats = useCategories()
  const bookmarks = useBookmarks()
  const searching = debouncedQ.trim().length > 1
  const search = useSearchBookmarks(debouncedQ, searching)

  const allTags = useMemo(() => collectTagOptions(bookmarks.data || []), [bookmarks.data])

  const catLabel = useMemo(() => {
    const list = cats.data || []
    const byId = new Map(list.map((c) => [c.id, c]))
    const map = new Map<string, string>()
    map.set(FOLDER_UNCATEGORIZED, 'Unsorted')
    for (const c of list) map.set(c.id, categoryPath(c, byId))
    return map
  }, [cats.data])

  const { list, searchCatchUpCount } = useMemo((): {
    list: LibraryListItem[]
    searchCatchUpCount: number
  } => {
    let catchUpCount = 0
    let raw: LibraryListItem[]
    if (searching) {
      const merged = mergeSearchWithLocalCatchUp(search.data?.hits || [], bookmarks.data || [], debouncedQ)
      raw = merged.items
      catchUpCount = merged.catchUpCount
    } else {
      raw = bookmarks.data || []
    }
    const flagsById = new Map((bookmarks.data || []).map((b) => [b.id, b]))
    const items = raw.map((item) => {
      const full = flagsById.get(item.id)
      if (!full) return item
      return {
        ...item,
        is_favorite: full.is_favorite,
        is_unread: full.is_unread,
        is_pinned: full.is_pinned,
        is_vault: full.is_vault,
        created_at: full.created_at,
        updated_at: full.updated_at,
        categories: full.categories,
      }
    })
    const catsForFilter =
      folderId === FOLDER_UNCATEGORIZED && !filterCats.includes(FOLDER_UNCATEGORIZED)
        ? [...filterCats, FOLDER_UNCATEGORIZED]
        : filterCats
    return {
      list: filterAndSortLibraryItems({
        items,
        filterCats: catsForFilter,
        filterTags,
        filterFlags,
        vaultOnly,
        sinceIso: sinceFilterToIso(filterSince),
        sort,
      }),
      searchCatchUpCount: catchUpCount,
    }
  }, [
    bookmarks.data,
    search.data,
    searching,
    debouncedQ,
    filterCats,
    filterTags,
    filterFlags,
    vaultOnly,
    filterSince,
    sort,
    folderId,
  ])

  const loading = bookmarks.isLoading || (searching && search.isFetching && !search.data)
  const emptyLibrary = !loading && !searching && !filterActive && list.length === 0
  const emptyFiltered = !loading && list.length === 0 && (searching || filterActive)
  const explorerLoading = bookmarks.isLoading || cats.isLoading

  return {
    cats,
    bookmarks,
    searching,
    search,
    allTags,
    catLabel,
    list,
    searchCatchUpCount,
    loading,
    emptyLibrary,
    emptyFiltered,
    explorerLoading,
  }
}
