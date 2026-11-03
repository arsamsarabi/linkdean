import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type {
  AdminEmailChangeResponse,
  AdminEmailVerifiedResponse,
  EmailChangeResponse,
  MeResponse,
  ResendVerificationResponse,
} from '@/schemas'

/** Asks once whether this browser holds a session; a 401 just means signed out. */
export function useSessionCheck(enabled = true): UseQueryResult<MeResponse> {
  return useQuery({ queryKey: ['me'], queryFn: api.me, enabled, retry: false })
}

export function useResendVerification(): UseMutationResult<ResendVerificationResponse, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.resendVerification(),
    onSuccess: (res) => {
      // Verified elsewhere (another tab, an admin): refresh so the banner or gate goes away.
      if (res.verified) void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useChangeEmail(): UseMutationResult<
  EmailChangeResponse,
  Error,
  { password: string; new_email: string }
> {
  return useMutation({ mutationFn: (body) => api.changeEmail(body) })
}

export function useSetEmailVerified(): UseMutationResult<
  AdminEmailVerifiedResponse,
  Error,
  { id: string; verified: boolean }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, verified }) => api.setEmailVerified(id, verified),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'users'] })
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export function useAdminChangeEmail(): UseMutationResult<
  AdminEmailChangeResponse,
  Error,
  { id: string; email: string }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, email }) => api.adminChangeEmail(id, email),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['admin', 'users'] })
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}
