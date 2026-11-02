import { describe, expect, it } from 'vitest'
import type { Bookmark, Category } from '@/schemas'
import { explorerSelectableItems } from './explorerSelection'

function bookmark(id: string, categoryId: string | null, flags: Partial<Bookmark> = {}): Bookmark {
  return {
    id,
    url: `https://example.com/${id}`,
    title: id,
    tags: [],
    category_id: categoryId,
    categories: categoryId ? [{ id: categoryId, name: categoryId, path: categoryId }] : [],
    is_vault: false,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...flags,
  } as Bookmark
}

const folder = { id: 'small', name: 'Small', parent_id: null } as Category
const library = [
  bookmark('in-1', 'small'),
  bookmark('in-2', 'small'),
  ...Array.from({ length: 50 }, (_, i) => bookmark(`other-${i}`, 'big')),
]

const base = {
  searching: false,
  searchHits: [],
  categories: [folder],
  bookmarks: library,
  showHidden: false,
  filterFlags: [],
  sort: 'title_asc' as const,
}

describe('explorerSelectableItems', () => {
  it('only reaches the open folder, not the whole library', () => {
    const ids = explorerSelectableItems({ ...base, folderId: 'small' }).map((b) => b.id)
    expect(ids.sort()).toEqual(['in-1', 'in-2'])
  })

  it('reaches nothing at the top level, which only shows folders', () => {
    expect(explorerSelectableItems({ ...base, folderId: 'all' })).toEqual([])
    expect(explorerSelectableItems({ ...base, folderId: undefined })).toEqual([])
  })

  it('reaches the search results while searching', () => {
    const hit = bookmark('hit', 'big')
    const ids = explorerSelectableItems({ ...base, folderId: 'small', searching: true, searchHits: [hit] })
    expect(ids.map((b) => b.id)).toEqual(['hit'])
  })

  it('respects flag filters, as the view does', () => {
    const bookmarks = [bookmark('fav', 'small', { is_favorite: true }), bookmark('plain', 'small')]
    const ids = explorerSelectableItems({ ...base, bookmarks, folderId: 'small', filterFlags: ['favorite'] })
    expect(ids.map((b) => b.id)).toEqual(['fav'])
  })
})
