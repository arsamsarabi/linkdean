import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import type { Bookmark } from '@/schemas'
import type { LibraryPageFilter } from './useLibraryBookmarksInfinite'
import { useLibrarySelection } from './useLibrarySelection'

vi.mock('@/api', () => ({ api: { bookmarkIds: vi.fn() } }))
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

  it('select all takes every id the filter matches, beyond the loaded rows', async () => {
    vi.mocked(api.bookmarkIds).mockResolvedValue(['a', 'b', 'far', 'ro'])
    const list = [item('a'), item('b'), { ...item('ro'), can_write: false } as unknown as Bookmark]
    const filter: LibraryPageFilter = { sort: 'updated_desc' }
    const { result, rerender } = renderHook(({ f }) => useLibrarySelection({ list, matchFilter: f }), {
      initialProps: { f: filter },
    })
    await act(() => result.current.toggleSelectAll())
    expect(api.bookmarkIds).toHaveBeenCalledWith(filter)
    expect([...result.current.selectedIds].sort()).toEqual(['a', 'b', 'far'])
    expect(result.current.allSelected).toBe(true)

    // Untick one: no longer everything.
    act(() => result.current.toggleSelect('a'))
    expect(result.current.allSelected).toBe(false)

    // A new filter drops the match-all selection down to what is on screen.
    await act(() => result.current.toggleSelectAll())
    rerender({ f: { sort: 'title_asc' } })
    expect([...result.current.selectedIds].sort()).toEqual(['a', 'b'])
  })

  it('select all falls back to the loaded rows when the screen is client-filtered', async () => {
    const list = [item('a'), item('b')]
    const { result } = renderHook(() => useLibrarySelection({ list, matchFilter: null }))
    await act(() => result.current.toggleSelectAll())
    expect([...result.current.selectedIds]).toEqual(['a', 'b'])
    await act(() => result.current.toggleSelectAll())
    expect(result.current.selectedCount).toBe(0)
  })
})
