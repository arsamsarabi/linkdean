import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { Notification } from '@/schemas'

export function useUnreadNotificationCount(enabled = true): UseQueryResult<{ count: number }> {
  return useQuery({
    queryKey: ['notifications', 'unread-count'],
    queryFn: () => api.unreadNotificationCount(),
    enabled,
    refetchInterval: 30_000,
  })
}

export function useNotifications(enabled = true): UseQueryResult<{ notifications: Notification[] }> {
  return useQuery({
    queryKey: ['notifications', 'list'],
    queryFn: () => api.listNotifications(50),
    enabled,
  })
}

export function useMarkNotificationRead(): UseMutationResult<
  { notification: Notification },
  Error,
  string
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.markNotificationRead(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['notifications'] })
    },
  })
}

export function useMarkAllNotificationsRead(): UseMutationResult<
  { ok: boolean; updated?: number },
  Error,
  void
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.markAllNotificationsRead(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['notifications'] })
    },
  })
}
