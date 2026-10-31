import { useMutation, useQuery, useQueryClient, type UseMutationResult, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { MeResponse, PlatformSettings, UserPublic, UserSettings } from '@/schemas'
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
  { registration_open?: boolean }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.updatePlatformSettings(body),
    onSuccess: (res) => {
      qc.setQueryData(['admin', 'platform-settings'], res)
      void qc.invalidateQueries({ queryKey: ['bootstrap'] })
    },
  })
}

export function useUpdateSettings(): UseMutationResult<SettingsEnvelope, Error, Partial<UserSettings>> {
  const qc = useQueryClient()
  const setShowHidden = useUi((s) => s.setShowHidden)
  return useMutation({
    mutationFn: (patch: Partial<UserSettings>) => api.updateSettings(patch),
    onSuccess: (res) => {
      qc.setQueryData(['me'], (old: MeResponse | undefined) => (old ? { ...old, settings: res.settings } : old))
      if (useUi.getState().showHidden !== res.settings.show_hidden) {
        setShowHidden(res.settings.show_hidden)
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
