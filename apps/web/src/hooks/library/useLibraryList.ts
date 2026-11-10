import { useEffect, useMemo } from 'react'
import { useSearchBookmarks, useTagNames } from '@/hooks/bookmarks'
import { useCategories } from '@/hooks/categories'
import { categoryPath, type SortKey } from '@/lib'
import {
  collectTagOptions,
  filterAndSortLibraryItems,
  FOLDER_UNCATEGORIZED,
  matchesVaultQuery,
  namedFolder,
  scopeBookmarks,
  scopeCategories,
  scopeToFolder,
  sinceFilterToIso,
  splitFolderFilter,
  type ExplorerFolderId,
  type FlagFilter,
  type LibraryListItem,
  type SinceFilter,
} from '@/services'
import { useUi } from '@/stores'
import type { LibraryScope } from './types'
import { useLibraryBookmarksInfinite, type LibraryPageFilter } from './useLibraryBookmarksInfinite'

type Input = {
  scope: LibraryScope
  vaultId: string | null
  showVaultInLibrary: boolean
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

/**
 * Keyset-paged bookmarks (the server filters and sorts), Meili search, and label maps.
 * `fetchNextPage` / `hasNextPage` drive infinite scroll.
 */
export function useLibraryList({
  scope,
  vaultId,
  showVaultInLibrary,
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
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const inVaults = scope === 'vault'
  const includeVault = inVaults || (vaultUnlocked && showVaultInLibrary)
  const catsQuery = useCategories(undefined, includeVault)
  const cats = useMemo(
    () => (inVaults ? { ...catsQuery, data: scopeCategories(catsQuery.data, vaultId) } : catsQuery),
    [catsQuery, inVaults, vaultId]
  ) as typeof catsQuery
  const searching = debouncedQ.trim().length > 1

  // `since` must be stable: a fresh timestamp per render would be a fresh query key.
  const sinceIso = useMemo(() => sinceFilterToIso(filterSince), [filterSince])
  // A fresh clientCats per render would rebuild the search list every render, and the selection's
  // render-time sync would loop on it.
  const { folder, clientCats } = useMemo(() => splitFolderFilter(folderId, filterCats), [folderId, filterCats])
  const filter = useMemo(
    (): LibraryPageFilter => ({
      folder,
      sort,
      tag: filterTags,
      flag: filterFlags,
      since: sinceIso,
      include_vault: includeVault,
      vault_only: inVaults || vaultOnly,
    }),
    [folder, sort, filterTags, filterFlags, sinceIso, includeVault, inVaults, vaultOnly]
  )
  // Meilisearch holds no vault content, so only the Vaults page searches the paged list.
  const infinite = useLibraryBookmarksInfinite(filter, !searching || inVaults)
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = infinite

  // ponytail: the API has no text or vault_id filter, so a vault search walks every page
  // (ceiling: vault size). Upgrade: a server `q` and `vault_id` on /bookmarks.
  useEffect(() => {
    if (inVaults && searching && hasNextPage && !isFetchingNextPage) void fetchNextPage()
  }, [inVaults, searching, hasNextPage, isFetchingNextPage, fetchNextPage])

  const loaded = useMemo(() => {
    const all = infinite.data?.pages.flatMap((p) => p.items) ?? []
    return inVaults ? scopeBookmarks(all, vaultId) : all
  }, [infinite.data, inVaults, vaultId])
  const bookmarks = useMemo(() => ({ ...infinite, data: loaded }), [infinite, loaded])
  const search = useSearchBookmarks(debouncedQ, searching && !inVaults)

  const tagNames = useTagNames()
  const allTags = useMemo(() => collectTagOptions([{ tags: tagNames.data ?? [] }, ...loaded]), [tagNames.data, loaded])

  // What a search finds, before list filters. Explorer shows these in place of a folder.
  const searchItems = useMemo((): LibraryListItem[] => {
    if (!searching) return []
    if (inVaults) return loaded.filter((b) => matchesVaultQuery(b, debouncedQ))
    return search.data?.hits || []
  }, [searching, inVaults, loaded, debouncedQ, search.data])

  const catLabel = useMemo(() => {
    const list = cats.data || []
    const byId = new Map(list.map((c) => [c.id, c]))
    const map = new Map<string, string>()
    map.set(FOLDER_UNCATEGORIZED, 'Unsorted')
    for (const c of list) map.set(c.id, categoryPath(c, byId))
    return map
  }, [cats.data])

  const list = useMemo((): LibraryListItem[] => {
    if (!searching) {
      // The server already filtered and sorted; only a category filter it cannot take is left.
      return clientCats.length
        ? filterAndSortLibraryItems({ items: loaded, filterCats: clientCats, filterTags: [], sort })
        : loaded
    }
    // ponytail: search hits are not server-filtered, and Meili catch-up is skipped (it needed every
    // bookmark loaded). Hits borrow flags from whatever pages are already loaded.
    const byId = new Map(loaded.map((b) => [b.id, b]))
    const items = searchItems.map((item) => {
      const full = byId.get(item.id)
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
    return filterAndSortLibraryItems({
      items: scopeToFolder(items, folderId),
      filterCats: catsForFilter,
      filterTags,
      filterFlags,
      vaultOnly,
      sinceIso,
      sort,
    })
  }, [
    searching,
    loaded,
    searchItems,
    clientCats,
    filterCats,
    filterTags,
    filterFlags,
    vaultOnly,
    sinceIso,
    sort,
    folderId,
  ])

  // The server's count is only the visible count when nothing is filtered client-side.
  const serverExact = !searching && !clientCats.length && !(inVaults && vaultId)
  const total = serverExact ? (infinite.data?.pages[0]?.total ?? 0) : list.length
  // What /bookmarks/ids can reproduce exactly; null when the screen is client-filtered.
  const matchFilter = serverExact ? filter : null

  const loading = bookmarks.isLoading || (searching && search.isFetching && !search.data)
  const inFolder = !!namedFolder(folderId)
  const emptyLibrary = !loading && !searching && !filterActive && !inFolder && list.length === 0
  const emptyFiltered = !loading && list.length === 0 && (searching || filterActive || inFolder)
  const explorerLoading = bookmarks.isLoading || cats.isLoading

  return {
    cats,
    bookmarks,
    searching,
    search,
    searchItems,
    allTags,
    catLabel,
    list,
    total,
    matchFilter,
    fetchNextPage,
    hasNextPage: !searching && hasNextPage,
    isFetchingNextPage,
    loading,
    emptyLibrary,
    emptyFiltered,
    explorerLoading,
  }
}
