import { useEffect, useSyncExternalStore } from 'react'

// Mantine closes every open modal and drawer on one Esc press unless they share a stack,
// so a confirm or crop dialog took its drawer, and the drawer's draft, down with it.
// Dialogs that open over a drawer register here; drawers skip Esc while any is open.
let openDialogs = 0
const listeners = new Set<() => void>()

function emit(): void {
  for (const l of listeners) l()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** Call in a dialog that can open over a drawer, with whether it is open. */
export function useChildDialog(opened: boolean): void {
  useEffect(() => {
    if (!opened) return
    openDialogs += 1
    emit()
    return () => {
      openDialogs -= 1
      emit()
    }
  }, [opened])
}

/** True while any modal dialog or drawer is on screen. Mantine renders both as `aria-modal`
 * dialogs, so page-level Esc shortcuts can leave the key to them. */
export function isModalDialogOpen(doc: Document = document): boolean {
  return doc.querySelector('[role="dialog"][aria-modal="true"]') !== null
}

/** For a drawer's `closeOnEscape`: false while a dialog is open on top of it. */
export function useDrawerClosesOnEscape(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => openDialogs === 0,
    () => true
  )
}
