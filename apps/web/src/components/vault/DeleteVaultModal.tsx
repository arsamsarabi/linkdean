import { useState, type ReactElement } from 'react'
import { Alert, Button, Modal, PasswordInput, Stack, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useDeleteVault } from '@/hooks'
import type { Vault } from '@/schemas'

export function DeleteVaultModal({ vault, onClose }: { vault: Vault; onClose: () => void }): ReactElement {
  const [name, setName] = useState('')
  const [password, setPassword] = useState('')
  const remove = useDeleteVault()

  async function onDelete(): Promise<void> {
    try {
      await remove.mutateAsync({ id: vault.id, name, password: password || undefined })
      notifications.show({ message: 'Vault deleted' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={`Delete ${vault.name}`}>
      <Stack>
        <Alert color="red" variant="light">
          This permanently deletes the vault with all its categories and bookmarks. They cannot be recovered.
        </Alert>
        <TextInput
          label={`Type ${vault.name} to confirm`}
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
        />
        <PasswordInput
          label="Account password"
          description="Leave empty if you sign in with single sign-on."
          value={password}
          onChange={(e) => setPassword(e.currentTarget.value)}
        />
        <Button color="red" disabled={name !== vault.name} loading={remove.isPending} onClick={() => void onDelete()}>
          Delete permanently
        </Button>
      </Stack>
    </Modal>
  )
}
