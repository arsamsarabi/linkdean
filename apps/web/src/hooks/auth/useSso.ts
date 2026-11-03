import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { MeResponse, PendingOidcLink, UserPublic } from '@/schemas'

/** The single sign-on waiting for a password sign-in in this browser. A 404 means there is none. */
export function usePendingOidcLink(enabled: boolean): UseQueryResult<PendingOidcLink> {
  return useQuery({ queryKey: ['oidc', 'pending-link'], queryFn: api.pendingOidcLink, enabled, retry: false })
}

export function useDisconnectSso(): UseMutationResult<{ ok: boolean; user: UserPublic }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (password: string) => api.disconnectSso(password),
    onSuccess: (res) => {
      qc.setQueryData<MeResponse>(['me'], (prev) => (prev ? { ...prev, user: res.user } : prev))
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}
