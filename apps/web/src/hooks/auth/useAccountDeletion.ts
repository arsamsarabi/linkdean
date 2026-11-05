import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { ScheduleDeletionResponse } from '@/schemas'

export function useScheduleAccountDeletion(): UseMutationResult<
  ScheduleDeletionResponse,
  Error,
  { confirm_email: string; password?: string }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.scheduleAccountDeletion(body),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['me'] }),
  })
}

export function useCancelAccountDeletion(): UseMutationResult<{ ok: boolean }, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.cancelAccountDeletion(),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['me'] }),
  })
}
