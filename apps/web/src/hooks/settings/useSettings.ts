import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type {
  AdminUserSummary,
  ApiKey,
  CreateApiKeyResponse,
  MeResponse,
  PlatformSettings,
  UserPublic,
  UserSettings,
} from '@/schemas'
import { useUi } from '@/stores'

type SettingsEnvelope = { settings: UserSettings }
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
  { registration_open?: boolean; api_keys_enabled?: boolean }
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
    mutationFn: ({ id, api_keys_enabled, is_admin }) =>
      api.updateAdminUser(id, { api_keys_enabled, is_admin }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'users'] })
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useReindexSearch(): UseMutationResult<
  { ok: boolean; upserted: number; failed: number },
  Error,
  void
> {
  return useMutation({
    mutationFn: () => api.reindexSearch(),
  })
}

export function useApiKeys(enabled = true): UseQueryResult<{ keys: ApiKey[] }> {
  return useQuery({
    queryKey: ['api-keys'],
    queryFn: () => api.listApiKeys(),
    enabled,
  })
}

export function useCreateApiKey(): UseMutationResult<CreateApiKeyResponse, Error, { name: string }> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.createApiKey(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}

export function useRevokeApiKey(): UseMutationResult<{ ok: boolean }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.revokeApiKey(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}

export function useUpdateSettings(): UseMutationResult<SettingsEnvelope, Error, Partial<UserSettings>> {
  const qc = useQueryClient()
  const setShowHidden = useUi((s) => s.setShowHidden)
  return useMutation({
    mutationFn: (patch: Partial<UserSettings>) => api.updateSettings(patch),
    onSuccess: (res, patch) => {
      qc.setQueryData(['me'], (old: MeResponse | undefined) => (old ? { ...old, settings: res.settings } : old))
      if (useUi.getState().showHidden !== res.settings.show_hidden) {
        setShowHidden(res.settings.show_hidden)
      }
      if (patch.show_deleted_in_library !== undefined) {
        void qc.invalidateQueries({ queryKey: ['bookmarks'] })
        void qc.invalidateQueries({ queryKey: ['categories'] })
        void qc.invalidateQueries({ queryKey: ['dashboards'] })
      }
    },
  })
}

type ProfileBody = {
  display_name?: string | null
  bio?: string | null
  timezone?: string | null
  email?: string
}

export function useUpdateProfile(): UseMutationResult<{ user: UserPublic }, Error, ProfileBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: ProfileBody) => api.updateProfile(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

type ChangePasswordBody = { current_password: string; new_password: string }

export function useChangePassword(): UseMutationResult<{ ok: boolean }, Error, ChangePasswordBody> {
  return useMutation({
    mutationFn: (body: ChangePasswordBody) => api.changePassword(body),
  })
}

export function useUploadAvatar(): UseMutationResult<{ user: UserPublic }, Error, Blob> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (blob: Blob) => api.uploadAvatar(blob),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useDeleteAvatar(): UseMutationResult<{ user: UserPublic }, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.deleteAvatar(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}
