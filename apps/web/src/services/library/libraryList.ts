import type { SortKey } from '@/lib'
import type { Bookmark, SearchHit } from '@/schemas'

export type LibraryListItem = Bookmark | (SearchHit & { categories?: Bookmark['categories'] })

export type LibraryListInput = {
  items: readonly LibraryListItem[]
  filterCats: readonly string[]
  filterTags: readonly string[]
  sort: SortKey
}

export function filterAndSortLibraryItems(input: LibraryListInput): LibraryListItem[] {
  let items: LibraryListItem[] = [...input.items]

  if (input.filterCats.length) {
    items = items.filter((b) => {
      const ids = new Set((b.categories || []).map((c) => c.id).concat(b.category_id ? [b.category_id] : []))
      return input.filterCats.some((id) => ids.has(id))
    })
  }
  if (input.filterTags.length) {
    items = items.filter((b) => input.filterTags.every((t) => (b.tags || []).includes(t)))
  }

  const sorted = [...items]
  sorted.sort((a, b) => {
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
