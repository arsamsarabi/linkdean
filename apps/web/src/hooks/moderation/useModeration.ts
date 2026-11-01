import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type {
  BanDurationBody,
  BanView,
  ExposedContent,
  ModerateItem,
  ModerateOutcome,
  ModerationAction,
} from '@/schemas'

function useBanMutation<T>(run: (input: T) => Promise<unknown>): UseMutationResult<unknown, Error, T> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: run,
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['admin'] }),
  })
}

export function useBans(enabled = true): UseQueryResult<{ bans: BanView[] }> {
  return useQuery({ queryKey: ['admin', 'bans'], queryFn: () => api.listBans(), enabled })
}

export const useBanUser = (): UseMutationResult<unknown, Error, { id: string; reason: string } & BanDurationBody> =>
  useBanMutation(({ id, ...body }) => api.banUser(id, body))

export const useChangeBans = (): UseMutationResult<
  unknown,
  Error,
  { user_ids: string[]; reason?: string } & BanDurationBody
> => useBanMutation((body) => api.changeBans(body))

export const useLiftBans = (): UseMutationResult<unknown, Error, string[]> => useBanMutation((ids) => api.liftBans(ids))

export function useExposedContent(userId: string | null): UseQueryResult<{ content: ExposedContent }> {
  return useQuery({
    queryKey: ['admin', 'exposed', userId],
    queryFn: () => api.exposedContent(userId as string),
    enabled: !!userId,
  })
}

export function useModerationActions(userId: string | null): UseQueryResult<{ actions: ModerationAction[] }> {
  return useQuery({
    queryKey: ['admin', 'moderation-actions', userId],
    queryFn: () => api.moderationActions(userId as string),
    enabled: !!userId,
  })
}

export function useModerate(
  userId: string
): UseMutationResult<{ outcome: ModerateOutcome }, Error, { reason: string; actions: ModerateItem[] }> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.moderate(userId, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'exposed', userId] })
      void qc.invalidateQueries({ queryKey: ['admin', 'moderation-actions', userId] })
    },
  })
}
