import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type {
  AdminUserSummary,
  CreateRegistrationInviteResponse,
  EmailStatus,
  PlatformSettings,
  RegistrationInvite,
  ScreenshotRetryStatus,
  TestEmailResult,
} from '@/schemas'

type PlatformEnvelope = {
  settings: PlatformSettings
  require_verified_email_forced: boolean
}

export function usePlatformSettings(enabled = true): UseQueryResult<PlatformEnvelope> {
  return useQuery({
    queryKey: ['admin', 'platform-settings'],
    queryFn: () => api.getPlatformSettings(),
    enabled,
  })
}

export type PlatformSettingsPatch = {
  registration_open?: boolean
  api_keys_enabled?: boolean
  require_verified_email?: boolean
  max_bookmarks?: number | null
  max_stored_image_bytes?: number | null
  max_enrich_actions_per_day?: number | null
  max_import_bytes?: number | null
  max_upload_image_bytes?: number | null
  abuse_contact_email?: string | null
  screenshot_retry_cooldown_hours?: number
  screenshot_auto_retry_days?: number
}

export function useUpdatePlatformSettings(): UseMutationResult<PlatformEnvelope, Error, PlatformSettingsPatch> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.updatePlatformSettings(body),
    onSuccess: (res) => {
      qc.setQueryData(['admin', 'platform-settings'], res)
      void qc.invalidateQueries({ queryKey: ['admin', 'screenshot-retry'] })
      void qc.invalidateQueries({ queryKey: ['bootstrap'] })
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useAdminUsers(enabled = true): UseQueryResult<{ users: AdminUserSummary[] }> {
  return useQuery({
    queryKey: ['admin', 'users'],
    queryFn: () => api.listAdminUsers(),
    enabled,
  })
}

export function useUpdateAdminUser(): UseMutationResult<
  { user: AdminUserSummary },
  Error,
  { id: string; api_keys_enabled?: boolean; is_admin?: boolean }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, api_keys_enabled, is_admin }) => api.updateAdminUser(id, { api_keys_enabled, is_admin }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'users'] })
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

const RETRY_KEY = ['admin', 'screenshot-retry'] as const

/** Failed screenshots and the retry cooldown. Polls while screenshots are being taken. */
export function useScreenshotRetryStatus(enabled = true): UseQueryResult<ScreenshotRetryStatus> {
  return useQuery({
    queryKey: RETRY_KEY,
    queryFn: () => api.screenshotRetryStatus(),
    enabled,
    refetchInterval: (q) => (q.state.data?.running ? 10_000 : false),
  })
}

export function useRetryScreenshots(): UseMutationResult<ScreenshotRetryStatus, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.retryScreenshots(),
    onSuccess: (res) => qc.setQueryData(RETRY_KEY, res),
    onSettled: () => void qc.invalidateQueries({ queryKey: ['admin', 'platform-settings'] }),
  })
}

export function useReindexSearch(): UseMutationResult<{ ok: boolean; upserted: number; failed: number }, Error, void> {
  return useMutation({
    mutationFn: () => api.reindexSearch(),
  })
}

export function useRegistrationInvites(enabled = true): UseQueryResult<{ invites: RegistrationInvite[] }> {
  return useQuery({
    queryKey: ['admin', 'invites'],
    queryFn: () => api.listRegistrationInvites(),
    enabled,
  })
}

export function useCreateRegistrationInvite(): UseMutationResult<
  CreateRegistrationInviteResponse,
  Error,
  { email?: string; expires_in_days?: number }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.createRegistrationInvite(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'invites'] })
    },
  })
}

export function useRevokeRegistrationInvite(): UseMutationResult<{ ok: boolean }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.revokeRegistrationInvite(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'invites'] })
    },
  })
}

export function useEmailStatus(enabled = true): UseQueryResult<EmailStatus> {
  return useQuery({
    queryKey: ['admin', 'email'],
    queryFn: () => api.getEmailStatus(),
    enabled,
    refetchInterval: 30_000,
  })
}

export function useSendTestEmail(): UseMutationResult<TestEmailResult, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.sendTestEmail(),
    onSettled: () => void qc.invalidateQueries({ queryKey: ['admin', 'email'] }),
  })
}
