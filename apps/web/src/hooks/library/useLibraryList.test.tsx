import { renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useLibraryList } from './useLibraryList'

const stable = vi.hoisted(() => ({
  search: { data: { hits: [{ id: 'a', title: 'Rust', url: 'https://rust-lang.org' }] }, isFetching: false },
  tags: { data: [] },
  cats: { data: [] },
  pages: { data: undefined, isLoading: false, hasNextPage: false, isFetchingNextPage: false, fetchNextPage: () => {} },
}))

vi.mock('@/hooks/bookmarks', () => ({ useSearchBookmarks: () => stable.search, useTagNames: () => stable.tags }))
vi.mock('@/hooks/categories', () => ({ useCategories: () => stable.cats }))
vi.mock('@/stores', () => ({
  useUi: (pick: (s: { vaultUnlocked: boolean }) => unknown) => pick({ vaultUnlocked: false }),
}))
vi.mock('./useLibraryBookmarksInfinite', () => ({ useLibraryBookmarksInfinite: () => stable.pages }))

const props = {
  scope: 'library' as const,
  vaultId: null,
  showVaultInLibrary: true,
  debouncedQ: 'rust',
  filterCats: [],
  filterTags: [],
  filterFlags: [],
  filterSince: null,
  vaultOnly: false,
  sort: 'updated_desc' as const,
  folderId: undefined,
  filterActive: false,
}

describe('useLibraryList', () => {
  it('keeps the search list stable across renders', () => {
    const { result, rerender } = renderHook((p) => useLibraryList(p), { initialProps: props })
    const first = result.current.list
    expect(first.map((b) => b.id)).toEqual(['a'])
    rerender(props)
    expect(result.current.list).toBe(first)
  })
})
