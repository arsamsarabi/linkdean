import { useState } from 'react'

/**
 * Runs `sync` during render on the first render and whenever `deps` change.
 * Use it to reset local state from props; `sync` may only set this component's state.
 */
export function useSyncDuringRender(deps: readonly unknown[], sync: () => void): void {
  const [seen, setSeen] = useState<readonly unknown[] | null>(null)
  if (seen && deps.every((dep, i) => Object.is(dep, seen[i]))) return
  setSeen(deps)
  sync()
}
