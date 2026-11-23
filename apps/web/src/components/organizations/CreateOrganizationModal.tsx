import { useState, type ReactElement } from 'react'
import { Button, Modal, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useNavigate } from '@tanstack/react-router'
import { useCreateOrganization } from '@/hooks'
import { useShellUi } from '@/stores'

type Props = {
  opened: boolean
  onClose: () => void
}

export function CreateOrganizationModal({ opened, onClose }: Props): ReactElement {
  const [name, setName] = useState('')
  const create = useCreateOrganization()
  const setOrg = useShellUi((s) => s.setOrganizationWorkspace)
  const nav = useNavigate()

  function reset(): void {
    setName('')
    onClose()
  }

  async function submit(): Promise<void> {
    const trimmed = name.trim()
    if (!trimmed) return
    try {
      const org = await create.mutateAsync(trimmed)
      notifications.show({ message: `Created ${org.name}` })
      setOrg(org.id, null)
      reset()
      void nav({ to: '/organizations/$id', params: { id: org.id } })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Modal opened={opened} onClose={reset} title="Create organization" centered>
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          Creates an organization with a default team. You become the owner.
        </Text>
        <TextInput
          label="Name"
          placeholder="Acme Inc"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void submit()
          }}
          data-autofocus
        />
        <Button loading={create.isPending} disabled={!name.trim()} onClick={() => void submit()}>
          Create
        </Button>
      </Stack>
    </Modal>
  )
}
