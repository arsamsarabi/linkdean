import { useInfiniteQuery, useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { FilingSuggestions } from '@/schemas'
import { FOLDER_UNCATEGORIZED } from '@/services'

/** Unsorted bookmarks, newest first, for the filing queue. Under `bookmarks` so edits refresh it. */
export function useUnsortedQueue() {
  return useInfiniteQuery({
    queryKey: ['bookmarks', 'filing-queue'],
    queryFn: ({ pageParam }) =>
      api.bookmarksPage({ folder: FOLDER_UNCATEGORIZED, sort: 'created_desc', limit: 50, cursor: pageParam }),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (last) => last.next_cursor ?? undefined,
  })
}

/** How many bookmarks are in Unsorted, for the sidebar. */
export function useUnsortedCount(enabled = true): UseQueryResult<number> {
  return useQuery({
    queryKey: ['bookmarks', 'unsorted-count'],
    queryFn: async () => (await api.bookmarksPage({ folder: FOLDER_UNCATEGORIZED, limit: 1 })).total,
    enabled,
    staleTime: 30_000,
  })
}

export function useFilingSuggestions(id: string | undefined): UseQueryResult<FilingSuggestions> {
  return useQuery({
    queryKey: ['filing-suggestions', id],
    queryFn: () => api.filingSuggestions(id!),
    enabled: !!id,
    staleTime: 60_000,
  })
}
