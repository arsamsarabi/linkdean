import { useEffect } from 'react'
import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api, ApiError } from '@/api'
import { isSignedIn, markSignedIn, sessionMarkerKey } from '@/lib'
import type { AuthTokenResponse, UserPublic } from '@/schemas'

type LoginBody = { email: string; password: string }
type RegisterBody = {
  email: string
  password: string
  display_name?: string
  invite_code?: string
  bootstrap_token?: string
  share_token?: string
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

// Known issue: Finishing onboarding can bounce back to it if the `me` refetch lands after
// navigation. Not reproduced. Fix: write the returned user into the cache before navigating.
export function useCompleteOnboarding(): UseMutationResult<{ user: UserPublic }, Error, void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.completeOnboarding(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['me'] })
    },
  })
}
