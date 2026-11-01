import { useState, type ReactElement } from 'react'
import { Alert, Button, Paper, PasswordInput, Stack, TextInput, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useCreateVault } from '@/hooks'
import { RecoveryKeyModal } from './RecoveryKeyModal'

export function CreateVaultForm(): ReactElement {
  const [name, setName] = useState('')
  const [passphrase, setPassphrase] = useState('')
  const [confirm, setConfirm] = useState('')
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null)
  const create = useCreateVault()
  const ready = !!name.trim() && passphrase.length >= 8 && passphrase === confirm

  async function onCreate(): Promise<void> {
    try {
      const res = await create.mutateAsync({ name, passphrase })
      setRecoveryKey(res.recovery_key)
      setName('')
      setPassphrase('')
      setConfirm('')
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack>
        <Title order={4}>New vault</Title>
        <TextInput label="Name" value={name} onChange={(e) => setName(e.currentTarget.value)} />
        <PasswordInput label="Passphrase" value={passphrase} onChange={(e) => setPassphrase(e.currentTarget.value)} />
        <PasswordInput label="Confirm passphrase" value={confirm} onChange={(e) => setConfirm(e.currentTarget.value)} />
        <Alert color="yellow" variant="light">
          If you lose both the passphrase and the recovery key, this vault's contents are gone for good.
        </Alert>
        <Button disabled={!ready} loading={create.isPending} onClick={() => void onCreate()}>
          Create vault
        </Button>
      </Stack>
      {recoveryKey ? <RecoveryKeyModal recoveryKey={recoveryKey} onDone={() => setRecoveryKey(null)} /> : null}
    </Paper>
  )
}
