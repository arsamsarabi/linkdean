import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { ShareInvite } from '@/schemas'

export function useShareInvites(enabled = true): UseQueryResult<{ invites: ShareInvite[] }> {
  return useQuery({
    queryKey: ['share-invites'],
    queryFn: () => api.shareInvites(),
    enabled,
    refetchInterval: 30_000,
  })
}

function useInviteAction(action: (id: string) => Promise<unknown>): UseMutationResult<unknown, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: action,
    onSuccess: () => {
      for (const key of ['share-invites', 'categories', 'bookmarks', 'dashboards', 'notifications']) {
        void qc.invalidateQueries({ queryKey: [key] })
      }
    },
  })
}

export function useAcceptShareInvite(): UseMutationResult<unknown, Error, string> {
  return useInviteAction(api.acceptShareInvite)
}

export function useRejectShareInvite(): UseMutationResult<unknown, Error, string> {
  return useInviteAction(api.rejectShareInvite)
}

export function useRestoreShareInvite(): UseMutationResult<unknown, Error, string> {
  return useInviteAction(api.restoreShareInvite)
}
