import { cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useApiImage } from './useApiImage'

const fetchMock = vi.fn()

describe('useApiImage', () => {
  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    URL.createObjectURL = vi.fn(() => 'blob:image')
    URL.revokeObjectURL = vi.fn()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
  })

  it('leaves external URLs alone', () => {
    const { result } = renderHook(() => useApiImage('https://example.com/a.png'))
    expect(result.current).toEqual({ src: 'https://example.com/a.png', failed: false })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  // Known issue: This failed once under a loaded full run and passed on rerun. Likely a timing
  // dependency; check before relying on it.
  it('loads API images with the session cookie', async () => {
    fetchMock.mockResolvedValue({ ok: true, blob: () => Promise.resolve(new Blob(['x'])) })
    const { result } = renderHook(() => useApiImage('/api/bookmarks/1/preview'))

    expect(result.current.src).toBeUndefined()
    await waitFor(() => expect(result.current.src).toBe('blob:image'))
    expect(fetchMock.mock.calls[0]?.[1]).toEqual({ credentials: 'include' })
  })

  it('reports a failed load', async () => {
    fetchMock.mockResolvedValue({ ok: false })
    const { result } = renderHook(() => useApiImage('/api/bookmarks/1/preview'))

    await waitFor(() => expect(result.current.failed).toBe(true))
    expect(result.current.src).toBeUndefined()
  })
})
