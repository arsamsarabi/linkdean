import { useState, type ReactElement } from 'react'
import { ConfirmModal } from './ConfirmModal'

type DiscardGuard = {
  /** Pass as the drawer's `onClose` and to any Cancel button. Saving calls `onClose` directly. */
  requestClose: () => void
  /** Render beside the drawer. */
  discardModal: ReactElement
}

/**
 * Asks before a form drawer with unsaved changes closes (Escape, a click outside, the close
 * button or Cancel). An unchanged form closes at once. The question is a child dialog, so
 * Escape answers it with Keep editing and leaves the drawer open.
 */
export function useDiscardGuard(dirty: boolean, onClose: () => void): DiscardGuard {
  const [asking, setAsking] = useState(false)

  function requestClose(): void {
    if (dirty) setAsking(true)
    else onClose()
  }

  const discardModal = (
    <ConfirmModal
      opened={asking}
      onClose={() => setAsking(false)}
      onConfirm={() => {
        setAsking(false)
        onClose()
      }}
      title="Discard changes?"
      body="Your changes in this form have not been saved."
      confirmLabel="Discard"
      cancelLabel="Keep editing"
    />
  )

  return { requestClose, discardModal }
}
