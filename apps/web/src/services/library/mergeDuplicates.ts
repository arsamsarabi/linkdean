import type { Bookmark } from '@/schemas'
import { unionTags } from './bulkSelection'

/** Prefer newest updated; ties → first in list. */
export function pickDefaultKeepId(bookmarks: readonly Bookmark[]): string | null {
  if (bookmarks.length === 0) return null
  let best = bookmarks[0]
  for (const b of bookmarks.slice(1)) {
    if (b.updated_at > best.updated_at) best = b
  }
  return best.id
}

export function mergeMeta(keep: Bookmark, losers: readonly Bookmark[]): {
  tags: string[]
  category_ids: string[]
} {
  let tags = [...keep.tags]
  const catIds = new Set((keep.categories || []).map((c) => c.id))
  for (const b of losers) {
    tags = unionTags(tags, b.tags)
    for (const c of b.categories || []) catIds.add(c.id)
  }
  return { tags, category_ids: [...catIds] }
}
