import { useInfiniteQuery } from '@tanstack/react-query'
import { api, type BookmarksListParams } from '@/api'

const PAGE_SIZE = 50

export type LibraryPageFilter = Omit<BookmarksListParams, 'cursor' | 'limit'>

/**
 * Keyset-paged library list. The filter (folder, sort, tags, flags, since, vault) is the
 * query key, so any filter change starts again from page one. Lives under `['bookmarks']`
 * so every bookmark mutation invalidates it.
 */
export function useLibraryBookmarksInfinite(filter: LibraryPageFilter, enabled: boolean) {
  return useInfiniteQuery({
    queryKey: ['bookmarks', 'library', filter] as const,
    initialPageParam: undefined as string | undefined,
    queryFn: ({ pageParam }) => api.bookmarksPage({ ...filter, cursor: pageParam, limit: PAGE_SIZE }),
    getNextPageParam: (last) => last.next_cursor ?? undefined,
    enabled,
  })
}
