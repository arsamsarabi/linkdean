import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api, ApiError } from '@/api'
import { getToken, setToken, tokenStorageKey } from '@/lib'
import { useLogin, useLogout, useSessionSync } from './useAuth'

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return {
    ...actual,
    api: {
      login: vi.fn().mockResolvedValue({ token: 'new', user: {} }),
      logout: vi.fn().mockResolvedValue({ ok: true }),
    },
  }
})

function wrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  }
}

beforeEach(() => setToken('old'))
afterEach(() => {
  setToken(null)
  vi.unstubAllGlobals()
})

describe('signing in', () => {
  it('drops the previous user cache on login', async () => {
    const qc = new QueryClient()
    qc.setQueryData(['bookmarks', true], [{ title: 'Previous user' }])
    const { result } = renderHook(() => useLogin(), { wrapper: wrapper(qc) })
    await act(() => result.current.mutateAsync({ email: 'a@b.c', password: 'password1' }))
    expect(qc.getQueryData(['bookmarks', true])).toBeUndefined()
    expect(getToken()).toBe('new')
  })
})

describe('signing out', () => {
  it('fails and keeps the session when the API could not end it', async () => {
    vi.mocked(api.logout).mockRejectedValueOnce(new ApiError('internal error', 500, {}))
    const qc = new QueryClient()
    qc.setQueryData(['me'], { user: {} })
    const { result } = renderHook(() => useLogout(), { wrapper: wrapper(qc) })
    await act(() => expect(result.current.mutateAsync()).rejects.toThrow('internal error'))
    expect(getToken()).toBe('old')
    expect(qc.getQueryData(['me'])).toBeDefined()
  })

  it('treats an already ended session as signed out', async () => {
    vi.mocked(api.logout).mockRejectedValueOnce(new ApiError('session expired', 401, {}))
    const qc = new QueryClient()
    const { result } = renderHook(() => useLogout(), { wrapper: wrapper(qc) })
    await act(() => result.current.mutateAsync())
    expect(getToken()).toBeNull()
  })
})

describe('other tabs', () => {
  it('goes to login when another tab signs out', () => {
    const assign = vi.fn()
    vi.stubGlobal('location', { pathname: '/library', assign })
    renderHook(() => useSessionSync())
    window.dispatchEvent(new StorageEvent('storage', { key: tokenStorageKey, newValue: null }))
    expect(assign).toHaveBeenCalledWith('/login')
  })

  it('reloads the app when another tab signs in as someone else', () => {
    const assign = vi.fn()
    vi.stubGlobal('location', { pathname: '/library', assign })
    renderHook(() => useSessionSync())
    window.dispatchEvent(new StorageEvent('storage', { key: 'unrelated', newValue: 'x' }))
    expect(assign).not.toHaveBeenCalled()
    window.dispatchEvent(new StorageEvent('storage', { key: tokenStorageKey, newValue: 'other' }))
    expect(assign).toHaveBeenCalledWith('/')
  })
})
