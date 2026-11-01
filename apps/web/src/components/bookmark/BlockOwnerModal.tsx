import type { ReactElement } from 'react'
import { ConfirmModal } from '@/components/confirm-modal'

type Props = {
  opened: boolean
  ownerLabel?: string | null
  loading: boolean
  onClose: () => void
  onConfirm: () => void
}

export function BlockOwnerModal({ opened, ownerLabel, loading, onClose, onConfirm }: Props): ReactElement {
  return (
    <ConfirmModal
      opened={opened}
      onClose={onClose}
      onConfirm={onConfirm}
      title={`Block ${ownerLabel || 'this user'}?`}
      body="Removes follows and shares between you, both ways, and hides each other's comments and notifications. They are not told. Unblocking does not bring shares back."
      confirmLabel="Block"
      loading={loading}
    />
  )
}
