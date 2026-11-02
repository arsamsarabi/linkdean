import { useState, type ReactElement } from 'react'
import { Button, Modal, PasswordInput, Stack, Textarea } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useRecoverVault } from '@/hooks'
import type { Vault } from '@/schemas'
import { MIN_SECRET_LENGTH } from '@/services'

export function RecoverVaultModal({ vault, onClose }: { vault: Vault; onClose: () => void }): ReactElement {
  const [recoveryKey, setRecoveryKey] = useState('')
  const [passphrase, setPassphrase] = useState('')
  const recover = useRecoverVault()

  async function onRecover(): Promise<void> {
    try {
      await recover.mutateAsync({ id: vault.id, recovery_key: recoveryKey, new_passphrase: passphrase })
      notifications.show({ message: `${vault.name} unlocked with a new passphrase` })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={`Recover ${vault.name}`}>
      <Stack>
        <Textarea label="Recovery key" value={recoveryKey} onChange={(e) => setRecoveryKey(e.currentTarget.value)} />
        <PasswordInput
          label="New passphrase"
          value={passphrase}
          onChange={(e) => setPassphrase(e.currentTarget.value)}
        />
        <Button
          disabled={!recoveryKey.trim() || passphrase.length < MIN_SECRET_LENGTH}
          loading={recover.isPending}
          onClick={() => void onRecover()}
        >
          Set new passphrase
        </Button>
      </Stack>
    </Modal>
  )
}
