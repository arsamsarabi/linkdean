import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useLibraryUrlState } from './useLibraryUrlState'

const router = vi.hoisted(() => ({ search: {} as Record<string, unknown> }))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => vi.fn(),
  useSearch: () => router.search,
}))

afterEach(() => {
  cleanup()
  router.search = {}
})

describe('useLibraryUrlState', () => {
  it('keeps a trailing space when the URL catches up with the trimmed query', () => {
    const { result, rerender } = renderHook(() => useLibraryUrlState('/library'))
    act(() => result.current.setQ('rust '))

    router.search = { q: 'rust' }
    rerender()

    expect(result.current.q).toBe('rust ')
  })

  it('takes a different query from the URL', () => {
    const { result, rerender } = renderHook(() => useLibraryUrlState('/library'))
    act(() => result.current.setQ('rust '))

    router.search = { q: 'go' }
    rerender()

    expect(result.current.q).toBe('go')
  })
})
