import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api } from '@/api'
import { markSignedIn } from '@/lib'
import type { AdminResetLink, AuthTokenResponse, ForgotPasswordResponse } from '@/schemas'

type ResetBody = { token: string; password: string; revoke_api_keys: boolean }

export function useForgotPassword(): UseMutationResult<ForgotPasswordResponse, Error, string> {
  return useMutation({ mutationFn: (email: string) => api.forgotPassword(email) })
}

/** Sets the new password and signs this browser in, like a login. */
export function useResetPassword(): UseMutationResult<AuthTokenResponse, Error, ResetBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: ResetBody) => api.resetPassword(body),
    onSuccess: () => {
      // Another user may have been signed in on this tab: drop everything cached for them.
      qc.clear()
      markSignedIn(true)
    },
  })
}

export function useCreateResetLink(): UseMutationResult<AdminResetLink, Error, string> {
  return useMutation({ mutationFn: (userId: string) => api.createResetLink(userId) })
}
