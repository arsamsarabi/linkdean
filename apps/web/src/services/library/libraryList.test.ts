import { describe, expect, it } from 'vitest'
import { FOLDER_UNCATEGORIZED } from './explorerLibrary'
import { collectTagOptions, filterAndSortLibraryItems, sinceFilterToIso } from './libraryList'
import { mergeSearchWithLocalCatchUp } from './searchCatchUp'

const base = {
  owner_user_id: 'u1',
  categories: [] as { id: string; name: string; path: string }[],
  description: null,
  notes: null as string | null,
  favicon_url: null,
  preview_image_url: null,
  has_preview: false,
  is_vault: false,
  can_write: true,
  shared_out: false,
  shared_with: [] as string[],
  is_favorite: false,
  is_unread: false,
  is_pinned: false,
  tags: [] as string[],
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-02T00:00:00Z',
}

describe('filterAndSortLibraryItems', () => {
  it('filters by category and tag then sorts by title', () => {
    const items = [
      {
        ...base,
        id: '1',
        url: 'https://a.example',
        title: 'Beta',
        category_id: 'c1',
        categories: [{ id: 'c1', name: 'C1', path: 'C1' }],
        tags: ['x'],
        updated_at: '2024-01-01T00:00:00Z',
      },
      {
        ...base,
        id: '2',
        url: 'https://b.example',
        title: 'Alpha',
        category_id: 'c1',
        categories: [{ id: 'c1', name: 'C1', path: 'C1' }],
        tags: ['x', 'y'],
        updated_at: '2024-01-03T00:00:00Z',
      },
      {
        ...base,
        id: '3',
        url: 'https://c.example',
        title: 'Gamma',
        category_id: 'c2',
        categories: [{ id: 'c2', name: 'C2', path: 'C2' }],
        tags: ['x'],
      },
    ]

    const result = filterAndSortLibraryItems({
      items,
      filterCats: ['c1'],
      filterTags: ['x'],
      sort: 'title_asc',
    })
    expect(result.map((i) => i.id)).toEqual(['2', '1'])
  })

  it('filters Unsorted (no categories)', () => {
    const items = [
      {
        ...base,
        id: '1',
        url: 'https://a.example',
        title: 'Filed',
        category_id: 'c1',
        categories: [{ id: 'c1', name: 'C1', path: 'C1' }],
      },
      {
        ...base,
        id: '2',
        url: 'https://b.example',
        title: 'Loose',
        category_id: null,
        categories: [],
      },
    ]

    const result = filterAndSortLibraryItems({
      items,
      filterCats: [FOLDER_UNCATEGORIZED],
      filterTags: [],
      sort: 'title_asc',
    })
    expect(result.map((i) => i.id)).toEqual(['2'])
  })

  it('filters flags and pins first', () => {
    const items = [
      {
        ...base,
        id: '1',
        url: 'https://a.example',
        title: 'Zulu',
        category_id: null,
        is_favorite: true,
        is_unread: false,
        is_pinned: false,
      },
      {
        ...base,
        id: '2',
        url: 'https://b.example',
        title: 'Alpha',
        category_id: null,
        is_favorite: true,
        is_unread: true,
        is_pinned: true,
      },
      {
        ...base,
        id: '3',
        url: 'https://c.example',
        title: 'Beta',
        category_id: null,
        is_favorite: false,
        is_unread: true,
        is_pinned: false,
      },
    ]
    const favorites = filterAndSortLibraryItems({
      items,
      filterCats: [],
      filterTags: [],
      filterFlags: ['favorite'],
      sort: 'title_asc',
    })
    expect(favorites.map((i) => i.id)).toEqual(['2', '1'])
    const unreadPinned = filterAndSortLibraryItems({
      items,
      filterCats: [],
      filterTags: [],
      filterFlags: ['unread'],
      sort: 'title_asc',
    })
    expect(unreadPinned.map((i) => i.id)).toEqual(['2', '3'])
  })
})

describe('collectTagOptions', () => {
  it('dedupes and sorts tags', () => {
    expect(collectTagOptions([{ tags: ['b', 'a'] }, { tags: ['a', 'c'] }, { tags: undefined }])).toEqual([
      { value: 'a', label: 'a' },
      { value: 'b', label: 'b' },
      { value: 'c', label: 'c' },
    ])
  })
})

describe('since + vault filters', () => {
  it('filters vault only and created since cutoff', () => {
    const now = Date.parse('2024-06-15T00:00:00Z')
    const items = [
      {
        ...base,
        id: '1',
        url: 'https://a.example',
        title: 'Old vault',
        category_id: null,
        is_vault: true,
        created_at: '2024-01-01T00:00:00Z',
      },
      {
        ...base,
        id: '2',
        url: 'https://b.example',
        title: 'New vault',
        category_id: null,
        is_vault: true,
        created_at: '2024-06-10T00:00:00Z',
      },
      {
        ...base,
        id: '3',
        url: 'https://c.example',
        title: 'New plain',
        category_id: null,
        is_vault: false,
        created_at: '2024-06-10T00:00:00Z',
      },
    ]
    const result = filterAndSortLibraryItems({
      items,
      filterCats: [],
      filterTags: [],
      vaultOnly: true,
      sinceIso: sinceFilterToIso('7d', now),
      sort: 'title_asc',
    })
    expect(result.map((i) => i.id)).toEqual(['2'])
  })
})

describe('mergeSearchWithLocalCatchUp', () => {
  it('appends local matches missing from Meili hits', () => {
    const hits = [
      {
        id: '1',
        owner_user_id: 'u1',
        title: 'Alpha',
        url: 'https://a.example',
        description: null,
        favicon_url: null,
        preview_image_url: null,
        has_preview: false,
        tags: [] as string[],
        category_id: null,
        is_vault: false,
        can_write: true,
      },
    ]
    const bookmarks = [
      {
        ...base,
        id: '1',
        url: 'https://a.example',
        title: 'Alpha',
        category_id: null,
      },
      {
        ...base,
        id: '2',
        url: 'https://b.example',
        title: 'Brand new alpha doc',
        category_id: null,
      },
      {
        ...base,
        id: '3',
        url: 'https://c.example',
        title: 'Other',
        category_id: null,
      },
    ]
    const { items, catchUpCount } = mergeSearchWithLocalCatchUp(hits, bookmarks, 'alpha')
    expect(catchUpCount).toBe(1)
    expect(items.map((i) => i.id)).toEqual(['1', '2'])
  })
})
