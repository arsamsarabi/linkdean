import { categoryPath } from '@/lib'
import type { Category } from '@/schemas'

/** How many folders the collapsed section shows. */
export const RECENT_FOLDER_COUNT = 3

function stamp(c: Category): string {
  return c.updated_at || c.created_at || ''
}

/**
 * The folders the collapsed section shows: the most recently updated ones, plus the open folder
 * when it is not among them, so you can always see where you are.
 */
export function recentFolders(cats: Category[], activeFolder: string | undefined): Category[] {
  const recent = [...cats].sort((a, b) => stamp(b).localeCompare(stamp(a))).slice(0, RECENT_FOLDER_COUNT)
  const active = activeFolder ? cats.find((c) => c.id === activeFolder) : undefined
  return active && !recent.includes(active) ? [...recent, active] : recent
}

/** Folders whose name holds `query`, any depth, with their full path, sorted by path. */
export function filterFolders(cats: Category[], query: string): { cat: Category; path: string }[] {
  const q = query.trim().toLowerCase()
  if (!q) return []
  const byId = new Map(cats.map((c) => [c.id, c]))
  return cats
    .filter((c) => c.name.toLowerCase().includes(q))
    .map((cat) => ({ cat, path: categoryPath(cat, byId) }))
    .sort((a, b) => a.path.localeCompare(b.path))
}
