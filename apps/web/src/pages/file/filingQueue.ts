import type { Bookmark } from '@/schemas'

/** The next bookmark to file: the first one not filed, skipped or deleted in this visit. */
export function currentCard(items: readonly Bookmark[], handled: ReadonlySet<string>): Bookmark | undefined {
  return items.find((b) => !handled.has(b.id))
}

/** Unsorted bookmarks not yet handled in this visit. Filed and deleted ones leave `total`
 *  once the list refetches; skipped ones stay in it, so they are taken off here. */
export function leftToFile(total: number, items: readonly Bookmark[], handled: ReadonlySet<string>): number {
  const handledHere = items.filter((b) => handled.has(b.id)).length
  return Math.max(0, total - handledHere)
}

export type QueueKey = { kind: 'file' } | { kind: 'skip' } | { kind: 'delete' } | { kind: 'folder'; index: number }

/**
 * What a key press does. Keys typed into a field are left alone, except Enter in the title
 * field, which files. Modifier keys never trigger anything.
 */
export function queueKey(
  e: { key: string; ctrlKey?: boolean; metaKey?: boolean; altKey?: boolean },
  where: 'page' | 'title' | 'field'
): QueueKey | null {
  if (e.ctrlKey || e.metaKey || e.altKey) return null
  if (where === 'title') return e.key === 'Enter' ? { kind: 'file' } : null
  if (where === 'field') return null
  if (e.key === 'Enter') return { kind: 'file' }
  const k = e.key.toLowerCase()
  if (k === 's') return { kind: 'skip' }
  if (k === 'd') return { kind: 'delete' }
  if (/^[1-5]$/.test(e.key)) return { kind: 'folder', index: Number(e.key) - 1 }
  return null
}

/** Add `id` when missing, remove it when present. */
export function toggle(ids: readonly string[], id: string): string[] {
  return ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]
}
