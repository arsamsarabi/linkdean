import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import { assertionToJSON, markSignedIn, registrationToJSON, toCreationOptions, toRequestOptions } from '@/lib'
import type { AuthTokenResponse, Passkey } from '@/schemas'

const KEY = ['passkeys'] as const

export function usePasskeys(enabled = true): UseQueryResult<{ passkeys: Passkey[] }> {
  return useQuery({ queryKey: KEY, queryFn: api.listPasskeys, enabled })
}

/** Runs the whole add ceremony: server options, the browser's passkey dialog, then save. */
export function useAddPasskey(): UseMutationResult<{ passkey: Passkey }, Error, { name: string; password?: string }> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async ({ name, password }) => {
      const start = await api.passkeyRegisterStart(password ? { password } : {})
      const cred = (await navigator.credentials.create(toCreationOptions(start.options))) as PublicKeyCredential | null
      if (!cred) throw new Error('No passkey was made')
      return api.passkeyRegisterFinish({ challenge_id: start.challenge_id, name, credential: registrationToJSON(cred) })
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: KEY }),
  })
}

export function useRenamePasskey(): UseMutationResult<{ ok: boolean }, Error, { id: string; name: string }> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, name }) => api.renamePasskey(id, name),
    onSuccess: () => void qc.invalidateQueries({ queryKey: KEY }),
  })
}

export function useDeletePasskey(): UseMutationResult<{ ok: boolean }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.deletePasskey(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: KEY }),
  })
}

/** Signs in with a passkey the browser holds for this site; no email needed. */
export function usePasskeyLogin(): UseMutationResult<AuthTokenResponse, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      const start = await api.passkeyLoginStart()
      const cred = (await navigator.credentials.get(toRequestOptions(start.options))) as PublicKeyCredential | null
      if (!cred) throw new Error('No passkey was chosen')
      return api.passkeyLoginFinish({ challenge_id: start.challenge_id, credential: assertionToJSON(cred) })
    },
    onSuccess: () => {
      // Another user may have been signed in on this tab: drop everything cached for them.
      qc.clear()
      markSignedIn(true)
    },
  })
}
