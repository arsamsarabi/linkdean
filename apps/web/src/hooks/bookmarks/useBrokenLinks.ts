import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Bookmark } from '@/schemas'

export function useBrokenLinks(enabled: boolean): UseQueryResult<Bookmark[]> {
  return useQuery({
    queryKey: ['broken-links'],
    queryFn: () => api.brokenBookmarks(),
    enabled,
  })
}

export function useRecheckBookmarkLink(): UseMutationResult<Bookmark, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.checkBookmarkLink(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['broken-links'] })
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
    },
  })
}
