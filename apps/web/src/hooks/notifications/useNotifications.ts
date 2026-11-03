import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Notification, NotificationEmailPref, NotificationKind } from '@/schemas'

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

export function useMarkNotificationRead(): UseMutationResult<{ notification: Notification }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.markNotificationRead(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['notifications'] })
    },
  })
}

export function useMarkAllNotificationsRead(): UseMutationResult<{ ok: boolean; updated?: number }, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.markAllNotificationsRead(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['notifications'] })
    },
  })
}

// Outside ['notifications'], so marking notifications read does not refetch it.
const EMAIL_PREFS_KEY = ['notification-email-prefs'] as const

export function useNotificationEmailPrefs(enabled = true): UseQueryResult<{ prefs: NotificationEmailPref[] }> {
  return useQuery({
    queryKey: EMAIL_PREFS_KEY,
    queryFn: () => api.notificationEmailPrefs(),
    enabled,
  })
}

type EmailPrefVars = { kind: NotificationKind; enabled: boolean }
type EmailPrefContext = { previous?: { prefs: NotificationEmailPref[] } }

/** Saves one kind's email switch, showing the new value at once and restoring it on failure. */
export function useSetNotificationEmailPref(): UseMutationResult<
  NotificationEmailPref,
  Error,
  EmailPrefVars,
  EmailPrefContext
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ kind, enabled }) => api.setNotificationEmailPref(kind, enabled),
    onMutate: async ({ kind, enabled }) => {
      await qc.cancelQueries({ queryKey: EMAIL_PREFS_KEY })
      const previous = qc.getQueryData<{ prefs: NotificationEmailPref[] }>(EMAIL_PREFS_KEY)
      if (previous) {
        qc.setQueryData(EMAIL_PREFS_KEY, {
          prefs: previous.prefs.map((p) => (p.kind === kind ? { ...p, enabled } : p)),
        })
      }
      return { previous }
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.previous) qc.setQueryData(EMAIL_PREFS_KEY, ctx.previous)
    },
    onSuccess: (saved) => {
      qc.setQueryData<{ prefs: NotificationEmailPref[] }>(EMAIL_PREFS_KEY, (cur) =>
        cur ? { prefs: cur.prefs.map((p) => (p.kind === saved.kind ? saved : p)) } : cur
      )
    },
  })
}
