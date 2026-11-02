import { useState, type ReactElement } from 'react'
import { Button, Modal, PasswordInput, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useUnlockAllVaults } from '@/hooks'

export function UnlockAllVaultsModal({ onClose }: { onClose: () => void }): ReactElement {
  const [passphrase, setPassphrase] = useState('')
  const unlock = useUnlockAllVaults()

  async function onUnlock(): Promise<void> {
    try {
      await unlock.mutateAsync(passphrase)
      notifications.show({ message: 'Vaults unlocked' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title="Unlock all">
      <Stack>
        <Text size="sm" c="dimmed">
          Opens every vault added to your global passphrase.
        </Text>
        <PasswordInput
          label="Global passphrase"
          value={passphrase}
          onChange={(e) => setPassphrase(e.currentTarget.value)}
        />
        <Button disabled={!passphrase} loading={unlock.isPending} onClick={() => void onUnlock()}>
          Unlock
        </Button>
      </Stack>
    </Modal>
  )
}
