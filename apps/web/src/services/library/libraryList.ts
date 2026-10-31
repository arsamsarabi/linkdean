import type { SortKey } from '@/lib'
import type { Bookmark, SearchHit } from '@/schemas'
import { FOLDER_UNCATEGORIZED, isUncategorized } from './explorerLibrary'

export type LibraryListItem = Bookmark | (SearchHit & { categories?: Bookmark['categories'] })

export const FLAG_FILTERS = ['unread', 'favorite', 'pinned'] as const
export type FlagFilter = (typeof FLAG_FILTERS)[number]

export type LibraryListInput = {
  items: readonly LibraryListItem[]
  filterCats: readonly string[]
  filterTags: readonly string[]
  filterFlags?: readonly FlagFilter[]
  sort: SortKey
}

function flagOf(item: LibraryListItem, key: 'is_favorite' | 'is_unread' | 'is_pinned'): boolean {
  return key in item ? Boolean((item as Bookmark)[key]) : false
}

export function filterAndSortLibraryItems(input: LibraryListInput): LibraryListItem[] {
  let items: LibraryListItem[] = [...input.items]

  if (input.filterCats.length) {
    const wantUnsorted = input.filterCats.includes(FOLDER_UNCATEGORIZED)
    const catIds = input.filterCats.filter((id) => id !== FOLDER_UNCATEGORIZED)
    items = items.filter((b) => {
      if (wantUnsorted && isUncategorized(b)) return true
      if (!catIds.length) return false
      const ids = new Set((b.categories || []).map((c) => c.id).concat(b.category_id ? [b.category_id] : []))
      return catIds.some((id) => ids.has(id))
    })
  }
  if (input.filterTags.length) {
    items = items.filter((b) => input.filterTags.every((t) => (b.tags || []).includes(t)))
  }
  if (input.filterFlags?.length) {
    items = items.filter((b) =>
      input.filterFlags!.every((f) => {
        if (f === 'unread') return flagOf(b, 'is_unread')
        if (f === 'favorite') return flagOf(b, 'is_favorite')
        return flagOf(b, 'is_pinned')
      })
    )
  }

  const sorted = [...items]
  sorted.sort((a, b) => {
    const ap = flagOf(a, 'is_pinned') ? 1 : 0
    const bp = flagOf(b, 'is_pinned') ? 1 : 0
    if (ap !== bp) return bp - ap

    const at = (a.title || a.url).toLowerCase()
    const bt = (b.title || b.url).toLowerCase()
    const au = 'updated_at' in a && a.updated_at ? a.updated_at : ''
    const bu = 'updated_at' in b && b.updated_at ? b.updated_at : ''
    const ac = 'created_at' in a && a.created_at ? a.created_at : ''
    const bc = 'created_at' in b && b.created_at ? b.created_at : ''
    switch (input.sort) {
      case 'title_asc':
        return at.localeCompare(bt)
      case 'title_desc':
        return bt.localeCompare(at)
      case 'updated_asc':
        return au.localeCompare(bu)
      case 'created_desc':
        return bc.localeCompare(ac)
      case 'updated_desc':
      default:
        return bu.localeCompare(au)
    }
  })
  return sorted
}

export function collectTagOptions(
  bookmarks: readonly { tags?: readonly string[] }[]
): { value: string; label: string }[] {
  const set = new Set<string>()
  for (const b of bookmarks) {
    for (const t of b.tags || []) set.add(t)
  }
  return [...set].sort().map((t) => ({ value: t, label: t }))
}
