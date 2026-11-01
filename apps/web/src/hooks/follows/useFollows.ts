import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { FollowRelation, FollowUser } from '@/schemas'

export const myFollowersQueryKey = ['follows', 'me', 'followers'] as const
export const myFollowingQueryKey = ['follows', 'me', 'following'] as const

export function followStatusQueryKey(userId: string): readonly ['follows', 'status', string] {
  return ['follows', 'status', userId] as const
}

export function useMyFollowers(enabled = true): UseQueryResult<{ users: FollowUser[] }> {
  return useQuery({
    queryKey: myFollowersQueryKey,
    queryFn: () => api.myFollowers(),
    enabled,
  })
}

export function useMyFollowing(enabled = true): UseQueryResult<{ users: FollowUser[] }> {
  return useQuery({
    queryKey: myFollowingQueryKey,
    queryFn: () => api.myFollowing(),
    enabled,
  })
}

export function useFollowStatus(
  userId: string | null | undefined,
  enabled = true
): UseQueryResult<{ follow: FollowRelation }> {
  return useQuery({
    queryKey: followStatusQueryKey(userId || ''),
    queryFn: () => api.followStatus(userId!),
    enabled: !!userId && enabled,
  })
}

function invalidateFollowQueries(qc: ReturnType<typeof useQueryClient>, userId?: string): void {
  void qc.invalidateQueries({ queryKey: myFollowersQueryKey })
  void qc.invalidateQueries({ queryKey: myFollowingQueryKey })
  void qc.invalidateQueries({ queryKey: ['follows', 'status'] })
  if (userId) void qc.invalidateQueries({ queryKey: followStatusQueryKey(userId) })
  void qc.invalidateQueries({ queryKey: ['notifications'] })
}

export function useFollowUser(): UseMutationResult<{ follow: FollowRelation }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.followUser(id),
    onSuccess: (_data, id) => invalidateFollowQueries(qc, id),
  })
}

export function useUnfollowUser(): UseMutationResult<{ follow: FollowRelation }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.unfollowUser(id),
    onSuccess: (_data, id) => invalidateFollowQueries(qc, id),
  })
}
