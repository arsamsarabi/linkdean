import { cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { setToken } from '@/lib'
import { useApiImage } from './useApiImage'

const fetchMock = vi.fn()

describe('useApiImage', () => {
  beforeEach(() => {
    fetchMock.mockReset()
    vi.stubGlobal('fetch', fetchMock)
    URL.createObjectURL = vi.fn(() => 'blob:image')
    URL.revokeObjectURL = vi.fn()
    setToken('page-token')
  })

  afterEach(() => {
    cleanup()
    setToken(null)
    vi.unstubAllGlobals()
  })

  it('leaves external URLs alone', () => {
    const { result } = renderHook(() => useApiImage('https://example.com/a.png'))
    expect(result.current).toEqual({ src: 'https://example.com/a.png', failed: false })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('loads API images with the page token', async () => {
    fetchMock.mockResolvedValue({ ok: true, blob: () => Promise.resolve(new Blob(['x'])) })
    const { result } = renderHook(() => useApiImage('/api/bookmarks/1/preview'))

    expect(result.current.src).toBeUndefined()
    await waitFor(() => expect(result.current.src).toBe('blob:image'))
    expect(fetchMock.mock.calls[0]?.[1]).toMatchObject({ headers: { Authorization: 'Bearer page-token' } })
  })

  it('reports a failed load', async () => {
    fetchMock.mockResolvedValue({ ok: false })
    const { result } = renderHook(() => useApiImage('/api/bookmarks/1/preview'))

    await waitFor(() => expect(result.current.failed).toBe(true))
    expect(result.current.src).toBeUndefined()
  })
})
