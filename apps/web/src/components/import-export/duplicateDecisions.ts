import type { DuplicateAction, DuplicateDecisions } from '@/schemas'

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
