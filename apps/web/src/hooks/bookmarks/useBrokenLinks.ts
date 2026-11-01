import {
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Bookmark } from '@/schemas'

export const brokenLinksQueryKey = ['broken-links'] as const

export function useBrokenLinks(enabled: boolean): UseQueryResult<Bookmark[]> {
  return useQuery({
    queryKey: brokenLinksQueryKey,
    queryFn: () => api.brokenBookmarks(),
    enabled,
    staleTime: 30_000,
  })
}

/** Prefetch broken-link list — call on header button hover/focus. */
export function usePrefetchBrokenLinks(): () => void {
  const qc = useQueryClient()
  return () => {
    void qc.prefetchQuery({
      queryKey: brokenLinksQueryKey,
      queryFn: () => api.brokenBookmarks(),
      staleTime: 30_000,
    })
  }
}

export function prefetchBrokenLinks(qc: QueryClient): void {
  void qc.prefetchQuery({
    queryKey: brokenLinksQueryKey,
    queryFn: () => api.brokenBookmarks(),
    staleTime: 30_000,
  })
}

export function useRecheckBookmarkLink(): UseMutationResult<Bookmark, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.checkBookmarkLink(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: brokenLinksQueryKey })
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
    },
  })
}
