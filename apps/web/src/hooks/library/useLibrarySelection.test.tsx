import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { Bookmark } from '@/schemas'
import { useLibrarySelection } from './useLibrarySelection'

vi.mock('@/hooks/bookmarks', () => {
  const idle = { isPending: false, mutateAsync: vi.fn() }
  return {
    useBulkAddCategories: () => idle,
    useBulkDeleteBookmarks: () => idle,
    useBulkTagBookmarks: () => idle,
  }
})

function item(id: string, is_vault = false): Bookmark {
  return { id, is_vault, access: 'owner' } as unknown as Bookmark
}

describe('library selection', () => {
  it('drops selected items that leave the screen', () => {
    const all = [item('a'), item('v', true)]
    const { result, rerender } = renderHook(({ list }) => useLibrarySelection({ list }), {
      initialProps: { list: all },
    })
    act(() => {
      result.current.toggleSelect('a')
      result.current.toggleSelect('v')
    })
    expect(result.current.selectedItems.map((b) => b.id)).toEqual(['a', 'v'])

    // A search narrows the list: the hidden vault item must not stay selected.
    rerender({ list: [item('a')] })
    expect([...result.current.selectedIds]).toEqual(['a'])
    expect(result.current.selectedItems.some((b) => b.is_vault)).toBe(false)
  })

  it('starts selecting when a box is ticked, without Select first', () => {
    const list = [item('a'), item('b')]
    const { result } = renderHook(() => useLibrarySelection({ list }))
    expect(result.current.selectMode).toBe(false)
    act(() => result.current.toggleSelect('b'))
    expect(result.current.selectMode).toBe(true)
    expect([...result.current.selectedIds]).toEqual(['b'])
  })

  it('selects the whole range with shift, in either direction', () => {
    const list = ['a', 'b', 'c', 'd', 'e'].map((id) => item(id))
    const { result } = renderHook(() => useLibrarySelection({ list }))
    act(() => result.current.toggleSelect('b'))
    act(() => result.current.toggleSelect('d', true))
    expect([...result.current.selectedIds].sort()).toEqual(['b', 'c', 'd'])
    act(() => result.current.exitSelectMode())
    act(() => result.current.toggleSelect('e'))
    act(() => result.current.toggleSelect('c', true))
    expect([...result.current.selectedIds].sort()).toEqual(['c', 'd', 'e'])
  })

  it('treats shift on the first tick as a plain tick', () => {
    const list = [item('a'), item('b')]
    const { result } = renderHook(() => useLibrarySelection({ list }))
    act(() => result.current.toggleSelect('b', true))
    expect([...result.current.selectedIds]).toEqual(['b'])
  })
})
