import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { SavedSearch, SavedSearchFilters } from '@/schemas'

const KEY = ['saved-searches'] as const

export function useSavedSearches(enabled = true): UseQueryResult<SavedSearch[]> {
  return useQuery({
    queryKey: KEY,
    queryFn: () => api.listSavedSearches(),
    enabled,
  })
}

export function useCreateSavedSearch(): UseMutationResult<
  SavedSearch,
  Error,
  { name: string; filters: SavedSearchFilters }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.createSavedSearch(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: KEY })
    },
  })
}

export function usePatchSavedSearch(): UseMutationResult<
  SavedSearch,
  Error,
  { id: string; name?: string; filters?: SavedSearchFilters }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...body }) => api.patchSavedSearch(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: KEY })
    },
  })
}

export function useDeleteSavedSearch(): UseMutationResult<{ ok: boolean }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.deleteSavedSearch(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: KEY })
    },
  })
}
