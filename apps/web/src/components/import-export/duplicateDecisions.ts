import type { DuplicateAction, DuplicateDecisions, ImportDuplicate } from '@/schemas'

/** A row's action: what the person picked for it, else the apply-to-all choice. */
export function actionFor(
  index: number,
  all: DuplicateAction,
  picked: ReadonlyMap<number, DuplicateAction>
): DuplicateAction {
  return picked.get(index) ?? all
}

/** The import body's `duplicates`: the default, plus only the rows that differ from it. */
export function decisionsBody(all: DuplicateAction, picked: ReadonlyMap<number, DuplicateAction>): DuplicateDecisions {
  const overrides: Record<number, DuplicateAction> = {}
  for (const [index, action] of picked) {
    if (action !== all) overrides[index] = action
  }
  return { default: all, overrides }
}

export type DuplicateTally = Record<DuplicateAction, number>

/** How many of `indexes` each action applies to, for the footer summary. */
export function tally(
  indexes: readonly number[],
  all: DuplicateAction,
  picked: ReadonlyMap<number, DuplicateAction>
): DuplicateTally {
  const out: DuplicateTally = { skip: 0, replace: 0, merge: 0, keep: 0 }
  for (const index of indexes) out[actionFor(index, all, picked)] += 1
  return out
}

export type DuplicateField = 'title' | 'description' | 'tags' | 'folders'

const norm = (s: string | null | undefined): string => (s ?? '').trim()
const sameSet = (a: readonly string[], b: readonly string[]): boolean =>
  a.length === b.length && [...a].sort().join('\u0000') === [...b].sort().join('\u0000')

/**
 * Fields where the file has a value that differs from the library's. A field the file leaves
 * empty never counts: Replace would not touch it.
 */
export function changedFields(d: ImportDuplicate): DuplicateField[] {
  const out: DuplicateField[] = []
  const { existing: a, incoming: b } = d
  if (norm(b.title) && norm(b.title) !== norm(a.title)) out.push('title')
  if (norm(b.description) && norm(b.description) !== norm(a.description)) out.push('description')
  if (b.tags.length && !sameSet(a.tags, b.tags)) out.push('tags')
  if (b.folders.length && !sameSet(a.folders, b.folders)) out.push('folders')
  return out
}
