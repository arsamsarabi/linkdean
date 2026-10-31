import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api } from '../../api'
import { setToken } from '../../lib/auth/authToken'
import type { AuthTokenResponse, UserPublic } from '../../schemas'

type LoginBody = { email: string; password: string }
type RegisterBody = { email: string; password: string; display_name?: string }

export function useLogin(): UseMutationResult<AuthTokenResponse, Error, LoginBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: LoginBody) => api.login(body),
    onSuccess: (res) => {
      setToken(res.token)
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useRegister(): UseMutationResult<AuthTokenResponse, Error, RegisterBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: RegisterBody) => api.register(body),
    onSuccess: (res) => {
      setToken(res.token)
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useLogout(): UseMutationResult<void, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      await api.logout().catch(() => undefined)
      setToken(null)
    },
    onSuccess: () => {
      qc.clear()
    },
  })
}

export function useCompleteOnboarding(): UseMutationResult<{ user: UserPublic }, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.completeOnboarding(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}
