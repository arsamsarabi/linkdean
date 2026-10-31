import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { TrashItem, TrashKind } from '@/schemas'

export function useTrash(): UseQueryResult<TrashItem[]> {
  return useQuery({
    queryKey: ['trash'],
    queryFn: () => api.trash(),
  })
}

function invalidateAll(qc: ReturnType<typeof useQueryClient>): void {
  void qc.invalidateQueries({ queryKey: ['trash'] })
  void qc.invalidateQueries({ queryKey: ['bookmarks'] })
  void qc.invalidateQueries({ queryKey: ['categories'] })
  void qc.invalidateQueries({ queryKey: ['dashboards'] })
  void qc.invalidateQueries({ queryKey: ['search'] })
  void qc.invalidateQueries({ queryKey: ['me'] })
}

export function useRestoreTrashItem(): UseMutationResult<
  { ok: boolean },
  Error,
  { kind: TrashKind; id: string }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ kind, id }) => api.restoreTrashItem(kind, id),
    onSuccess: () => invalidateAll(qc),
  })
}

export function usePurgeTrashItem(): UseMutationResult<
  { ok: boolean },
  Error,
  { kind: TrashKind; id: string }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ kind, id }) => api.purgeTrashItem(kind, id),
    onSuccess: () => invalidateAll(qc),
  })
}

export function usePurgeTrashSelected(): UseMutationResult<
  { ok: boolean; purged?: number; failed?: number },
  Error,
  Array<{ kind: TrashKind; id: string }>
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (items) => api.purgeTrashSelected(items),
    onSuccess: () => invalidateAll(qc),
  })
}
