import type { SortKey } from '@/lib'
import type { Bookmark, SearchHit } from '@/schemas'
import { FOLDER_ALL, FOLDER_UNCATEGORIZED, isUncategorized } from './explorerLibrary'

export type LibraryListItem = Bookmark | (SearchHit & { categories?: Bookmark['categories'] })

export const FLAG_FILTERS = ['unread', 'favorite', 'pinned'] as const
export type FlagFilter = (typeof FLAG_FILTERS)[number]

export const SINCE_FILTERS = ['7d', '30d', '90d', 'year'] as const
export type SinceFilter = (typeof SINCE_FILTERS)[number]

export type LibraryListInput = {
  items: readonly LibraryListItem[]
  filterCats: readonly string[]
  filterTags: readonly string[]
  filterFlags?: readonly FlagFilter[]
  /** Keep only vault bookmarks when true. */
  vaultOnly?: boolean
  /** Keep items created on/after this ISO cutoff (usually from a since preset). */
  sinceIso?: string | null
  sort: SortKey
}

function flagOf(item: LibraryListItem, key: 'is_favorite' | 'is_unread' | 'is_pinned'): boolean {
  return key in item ? Boolean((item as Bookmark)[key]) : false
}

function createdAtOf(item: LibraryListItem): string {
  return 'created_at' in item && item.created_at ? item.created_at : ''
}

function isVaultOf(item: LibraryListItem): boolean {
  return 'is_vault' in item ? Boolean(item.is_vault) : false
}

/** Resolve a since preset to an ISO timestamp relative to `now`. */
export function sinceFilterToIso(since: SinceFilter | null | undefined, now = Date.now()): string | null {
  if (!since) return null
  const dayMs = 24 * 60 * 60 * 1000
  const days = since === '7d' ? 7 : since === '30d' ? 30 : since === '90d' ? 90 : 365
  return new Date(now - days * dayMs).toISOString()
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
  if (input.vaultOnly) {
    items = items.filter((b) => isVaultOf(b))
  }
  if (input.sinceIso) {
    const cutoff = input.sinceIso
    items = items.filter((b) => {
      const created = createdAtOf(b)
      return created && created >= cutoff
    })
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

/** The folder the URL names, or null for All and Unsorted, which have their own handling. */
export function namedFolder(folderId: string | null | undefined): string | null {
  return folderId && folderId !== FOLDER_ALL && folderId !== FOLDER_UNCATEGORIZED ? folderId : null
}

/** Items filed directly in `folderId`, or all items when no folder is named. */
export function scopeToFolder<T extends LibraryListItem>(items: T[], folderId: string | null | undefined): T[] {
  const folder = namedFolder(folderId)
  if (!folder) return items
  return items.filter((b) => b.category_id === folder || (b.categories || []).some((c) => c.id === folder))
}
