import { useEffect, useState, type ReactElement } from 'react'
import { Button, Group, Modal, Stack, TextInput } from '@mantine/core'
import { useChildDialog } from '@/hooks/render'

type Props = {
  opened: boolean
  onClose: () => void
  title?: string
  initialName?: string
  confirmLabel?: string
  loading?: boolean
  onSubmit: (name: string) => void | Promise<void>
}

export function SaveSearchModal({
  opened,
  onClose,
  title = 'Save search',
  initialName = '',
  confirmLabel = 'Save',
  loading,
  onSubmit,
}: Props): ReactElement {
  const [name, setName] = useState(initialName)
  useChildDialog(opened)

  useEffect(() => {
    if (opened) setName(initialName)
  }, [opened, initialName])

  async function submit(): Promise<void> {
    const trimmed = name.trim()
    if (!trimmed || loading) return
    await onSubmit(trimmed)
  }

  return (
    <Modal opened={opened} onClose={onClose} title={title} centered>
      <Stack gap="md">
        <TextInput
          label="Name"
          placeholder="e.g. Unread Rust docs"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void submit()
          }}
          data-autofocus
          maxLength={80}
        />
        <Group justify="flex-end" gap="sm">
          <Button variant="default" onClick={onClose} disabled={loading}>
            Cancel
          </Button>
          <Button loading={loading} disabled={!name.trim()} onClick={() => void submit()}>
            {confirmLabel}
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
