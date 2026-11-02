import { useEffect } from 'react'
import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api, ApiError } from '@/api'
import { setToken, tokenStorageKey } from '@/lib'
import type { AuthTokenResponse, UserPublic } from '@/schemas'

type LoginBody = { email: string; password: string }
type RegisterBody = {
  email: string
  password: string
  display_name?: string
  invite_code?: string
  share_token?: string
}

export function useLogin(): UseMutationResult<AuthTokenResponse, Error, LoginBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: LoginBody) => api.login(body),
    onSuccess: (res) => {
      // Another user may have been signed in on this tab: drop everything cached for them.
      qc.clear()
      setToken(res.token)
    },
  })
}

export function useRegister(): UseMutationResult<AuthTokenResponse, Error, RegisterBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: RegisterBody) => api.register(body),
    onSuccess: (res) => {
      qc.clear()
      setToken(res.token)
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
      setToken(null)
    },
    onSuccess: () => {
      qc.clear()
    },
  })
}

/** Follows sign-in and sign-out in other tabs, so no tab keeps showing another session's data. */
export function useSessionSync(): void {
  useEffect(() => {
    function onStorage(e: StorageEvent): void {
      if (e.key !== tokenStorageKey && e.key !== null) return
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
