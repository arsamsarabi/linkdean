import { useEffect } from 'react'
import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api, ApiError } from '@/api'
import { isSignedIn, markSignedIn, sessionMarkerKey } from '@/lib'
import type { AuthTokenResponse, MeResponse, UserPublic } from '@/schemas'

type LoginBody = { email: string; password: string }
type RegisterBody = {
  email: string
  password: string
  display_name?: string
  invite_code?: string
  bootstrap_token?: string
  share_token?: string
  turnstile_token?: string
  terms_version: string
}

export function useLogin(): UseMutationResult<AuthTokenResponse, Error, LoginBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: LoginBody) => api.login(body),
    onSuccess: () => {
      // Another user may have been signed in on this tab: drop everything cached for them.
      qc.clear()
      markSignedIn(true)
    },
  })
}

export function useRegister(): UseMutationResult<AuthTokenResponse, Error, RegisterBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: RegisterBody) => api.register(body),
    onSuccess: () => {
      qc.clear()
      markSignedIn(true)
    },
  })
}

/** Fails, keeping the session, when the API could not end it. A 401 means it had already ended. */
export function useLogout(): UseMutationResult<void, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      try {
        await api.logout()
      } catch (e) {
        if (!(e instanceof ApiError && e.status === 401)) throw e
      }
      markSignedIn(false)
    },
    // The cache is dropped on the sign-in page (`useDropSignedOutCache`), not here: clearing it
    // while the app is still on screen made every open query refetch with the session gone.
    onSuccess: () => qc.cancelQueries(),
  })
}

/** On the sign-in page, drop what the last session left in the cache. */
export function useDropSignedOutCache(): void {
  const qc = useQueryClient()
  useEffect(() => {
    if (!isSignedIn()) qc.clear()
  }, [qc])
}

/** Follows sign-in and sign-out in other tabs, so no tab keeps showing another session's data. */
export function useSessionSync(): void {
  useEffect(() => {
    function onStorage(e: StorageEvent): void {
      if (e.key !== sessionMarkerKey && e.key !== null) return
      location.assign(e.newValue ? '/' : '/login')
    }
    window.addEventListener('storage', onStorage)
    return () => window.removeEventListener('storage', onStorage)
  }, [])
}

/**
 * Writes the onboarded user into the `me` cache before the mutation resolves. The app shell
 * sends anyone whose cached user is not onboarded back to /onboarding, and callers navigate
 * as soon as this resolves, long before a refetch would land.
 */
export function useCompleteOnboarding(): UseMutationResult<{ user: UserPublic }, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.completeOnboarding(),
    onSuccess: (res) => {
      qc.setQueryData(['me'], (old: MeResponse | undefined) => (old ? { ...old, user: res.user } : old))
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}
