import type { Bookmark, SearchHit } from '@/schemas'
import type { LibraryListItem } from './libraryList'

function haystack(b: Pick<Bookmark, 'title' | 'url' | 'description' | 'notes' | 'tags'>): string {
  return [b.title, b.url, b.description || '', b.notes || '', ...(b.tags || [])].join(' ').toLowerCase()
}

function matchesQuery(b: Bookmark, query: string): boolean {
  const q = query.trim().toLowerCase()
  if (q.length < 2) return false
  const tokens = q.split(/\s+/).filter(Boolean)
  const hay = haystack(b)
  return tokens.every((t) => hay.includes(t))
}

/**
 * When Meili lags behind writes, merge local bookmarks that match `query`
 * but are missing from search hits. Returns merged list + count of catch-up rows.
 */
export function mergeSearchWithLocalCatchUp(
  hits: readonly SearchHit[],
  bookmarks: readonly Bookmark[],
  query: string
): { items: LibraryListItem[]; catchUpCount: number } {
  const hitIds = new Set(hits.map((h) => h.id))
  const extras = bookmarks.filter((b) => !hitIds.has(b.id) && matchesQuery(b, query))
  if (extras.length === 0) {
    return { items: [...hits], catchUpCount: 0 }
  }
  return { items: [...hits, ...extras], catchUpCount: extras.length }
}
