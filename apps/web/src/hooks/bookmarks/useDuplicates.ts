import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Bookmark, DuplicatesResponse } from '@/schemas'
import { mergeMeta } from '@/services'

export function useDuplicates(enabled: boolean): UseQueryResult<DuplicatesResponse> {
  return useQuery({
    queryKey: ['duplicates'],
    queryFn: () => api.duplicates(),
    enabled,
  })
}

export type MergeDuplicateVars = {
  keep: Bookmark
  losers: Bookmark[]
}

export function useMergeDuplicateGroup(): UseMutationResult<void, Error, MergeDuplicateVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async ({ keep, losers }: MergeDuplicateVars) => {
      const { tags, category_ids } = mergeMeta(keep, losers)
      await api.updateBookmark(keep.id, { tags, category_ids })
      for (const b of losers) {
        await api.deleteBookmark(b.id)
      }
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      void qc.invalidateQueries({ queryKey: ['duplicates'] })
    },
  })
}
