import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useLockAllVaults, useLockVault } from './useVault'

vi.mock('@/api', () => ({
  api: {
    lockVault: vi.fn().mockResolvedValue({ ok: true }),
    lockAllVaults: vi.fn().mockResolvedValue({ ok: true }),
  },
}))

function seeded(): QueryClient {
  const qc = new QueryClient()
  qc.setQueryData(['bookmarks', true], [{ title: 'Secret' }])
  qc.setQueryData(['categories', false, true], [{ name: 'Secret' }])
  qc.setQueryData(['search', 'secret'], { hits: [{ title: 'Secret' }] })
  qc.setQueryData(['dashboard', 'default'], { title: 'Secret' })
  return qc
}

function expectNothingCached(qc: QueryClient): void {
  expect(qc.getQueryData(['bookmarks', true])).toBeUndefined()
  expect(qc.getQueryData(['categories', false, true])).toBeUndefined()
  expect(qc.getQueryData(['search', 'secret'])).toBeUndefined()
  expect(qc.getQueryData(['dashboard', 'default'])).toBeUndefined()
}

function wrapper(qc: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
  }
}

describe('locking vaults', () => {
  it('drops decrypted data when one vault locks', async () => {
    const qc = seeded()
    const { result } = renderHook(() => useLockVault(), { wrapper: wrapper(qc) })
    await act(() => result.current.mutateAsync('v1'))
    expectNothingCached(qc)
  })

  it('drops decrypted data when all vaults lock', async () => {
    const qc = seeded()
    const { result } = renderHook(() => useLockAllVaults(), { wrapper: wrapper(qc) })
    await act(() => result.current.mutateAsync())
    expectNothingCached(qc)
  })
})
