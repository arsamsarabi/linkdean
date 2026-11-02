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
    const { result, rerender } = renderHook(({ list }) => useLibrarySelection({ list, bookmarks: all }), {
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
})
