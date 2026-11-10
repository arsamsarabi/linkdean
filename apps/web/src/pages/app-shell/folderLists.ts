import type { Category } from '@/schemas'

/** How many folders the collapsed section shows. */
export const RECENT_FOLDER_COUNT = 3

/**
 * Latest activity: a bookmark added or changed in the folder, or the folder itself being made
 * or renamed, so a new empty folder still shows up. ISO strings from the API compare as text.
 */
function stamp(c: Category): string {
  const own = c.updated_at || c.created_at || ''
  const bookmarks = c.last_bookmark_at || ''
  return bookmarks > own ? bookmarks : own
}

/**
 * The folders the collapsed section shows: the most recently active ones, plus the open folder
 * when it is not among them, so you can always see where you are.
 */
export function recentFolders(cats: Category[], activeFolder: string | undefined): Category[] {
  const recent = [...cats].sort((a, b) => stamp(b).localeCompare(stamp(a))).slice(0, RECENT_FOLDER_COUNT)
  const active = activeFolder ? cats.find((c) => c.id === activeFolder) : undefined
  return active && !recent.includes(active) ? [...recent, active] : recent
}
