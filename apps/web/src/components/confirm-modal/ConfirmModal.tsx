import type { ReactElement } from 'react'
import { Button, Group, Modal, Stack, Text } from '@mantine/core'
import { useChildDialog } from '@/hooks/render'

type Props = {
  opened: boolean
  onClose: () => void
  onConfirm: () => void
  title: string
  body: string
  confirmLabel?: string
  confirmColor?: string
  loading?: boolean
}

export function ConfirmModal({
  opened,
  onClose,
  onConfirm,
  title,
  body,
  confirmLabel = 'Confirm',
  confirmColor = 'red',
  loading,
}: Props): ReactElement {
  useChildDialog(opened)
  return (
    <Modal opened={opened} onClose={onClose} title={title} centered>
      <Stack gap="lg">
        <Text size="sm">{body}</Text>
        <Group justify="flex-end" gap="sm">
          <Button variant="default" onClick={onClose} disabled={loading}>
            Cancel
          </Button>
          <Button color={confirmColor} onClick={onConfirm} loading={loading}>
            {confirmLabel}
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
