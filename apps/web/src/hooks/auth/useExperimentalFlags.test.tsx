import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import { useExperimentalFlags } from './useExperimentalFlags'

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
}

describe('useExperimentalFlags', () => {
  it('reads every flag off until the server answers, then uses its answer', async () => {
    let answer: (v: Awaited<ReturnType<typeof api.authConfig>>) => void = () => {}
    vi.spyOn(api, 'authConfig').mockReturnValue(new Promise((r) => (answer = r)))
    const { result } = renderHook(() => useExperimentalFlags(), { wrapper })
    expect(result.current.agent_skills).toBe(false)
    answer({ experimental: { agent_skills: true } })
    await waitFor(() => expect(result.current.agent_skills).toBe(true))
  })

  it('stays off when the request fails', async () => {
    const spy = vi.spyOn(api, 'authConfig').mockRejectedValue(new Error('down'))
    const { result } = renderHook(() => useExperimentalFlags(), { wrapper })
    await waitFor(() => expect(spy).toHaveBeenCalled())
    expect(result.current.agent_skills).toBe(false)
  })
})
