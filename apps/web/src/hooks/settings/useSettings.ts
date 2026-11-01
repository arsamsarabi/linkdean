import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { ApiKey, CreateApiKeyResponse, MeResponse, ProfileVisibility, UserPublic, UserSettings } from '@/schemas'
import { useUi } from '@/stores'

type SettingsEnvelope = { settings: UserSettings }

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
  profile_visibility?: ProfileVisibility
}

export function useUpdateProfile(): UseMutationResult<{ user: UserPublic }, Error, ProfileBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: ProfileBody) => api.updateProfile(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['me'] })
      void qc.invalidateQueries({ queryKey: ['profiles'] })
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
