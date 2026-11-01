import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { BlockRelation, FollowUser, MuteRelation } from '@/schemas'

export const myBlockedQueryKey = ['blocks', 'me', 'blocked'] as const
export const myMutedQueryKey = ['blocks', 'me', 'muted'] as const

export function useMyBlocked(enabled = true): UseQueryResult<{ users: FollowUser[] }> {
  return useQuery({
    queryKey: myBlockedQueryKey,
    queryFn: () => api.myBlocked(),
    enabled,
  })
}

export function useMyMuted(enabled = true): UseQueryResult<{ users: FollowUser[] }> {
  return useQuery({
    queryKey: myMutedQueryKey,
    queryFn: () => api.myMuted(),
    enabled,
  })
}

/** Block and mute reach shares, follows, notifications and social, so refetch everything. */
function invalidateAll(qc: ReturnType<typeof useQueryClient>): void {
  void qc.invalidateQueries()
}

export function useBlockUser(): UseMutationResult<{ block: BlockRelation }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.blockUser(id),
    onSuccess: () => invalidateAll(qc),
  })
}

export function useUnblockUser(): UseMutationResult<{ block: BlockRelation }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.unblockUser(id),
    onSuccess: () => invalidateAll(qc),
  })
}

export function useMuteUser(): UseMutationResult<{ mute: MuteRelation }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.muteUser(id),
    onSuccess: () => invalidateAll(qc),
  })
}

export function useUnmuteUser(): UseMutationResult<{ mute: MuteRelation }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.unmuteUser(id),
    onSuccess: () => invalidateAll(qc),
  })
}
