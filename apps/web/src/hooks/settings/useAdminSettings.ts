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
  TestEmailResult,
} from '@/schemas'

type PlatformEnvelope = { settings: PlatformSettings }

export function usePlatformSettings(enabled = true): UseQueryResult<PlatformEnvelope> {
  return useQuery({
    queryKey: ['admin', 'platform-settings'],
    queryFn: () => api.getPlatformSettings(),
    enabled,
  })
}

export function useUpdatePlatformSettings(): UseMutationResult<
  PlatformEnvelope,
  Error,
  { registration_open?: boolean; api_keys_enabled?: boolean; require_verified_email?: boolean }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.updatePlatformSettings(body),
    onSuccess: (res) => {
      qc.setQueryData(['admin', 'platform-settings'], res)
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
