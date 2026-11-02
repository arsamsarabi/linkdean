import { useState, type ReactElement } from 'react'
import {
  Badge,
  Button,
  Group,
  Modal,
  Paper,
  PasswordInput,
  Skeleton,
  Stack,
  Switch,
  Text,
  TextInput,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLock, IconLockOpen } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  useGlobalPassphrase,
  useJoinGlobal,
  useLeaveGlobal,
  useLockVault,
  useNewRecoveryKey,
  useRenameVault,
  useUnlockVault,
  useVaults,
} from '@/hooks'
import type { Vault } from '@/schemas'
import { DeleteVaultModal } from './DeleteVaultModal'
import { RecoverVaultModal } from './RecoverVaultModal'
import { RecoveryKeyModal } from './RecoveryKeyModal'

const iconSm = { size: 16, stroke: 1.5 } as const

async function report(action: Promise<unknown>, message: string): Promise<void> {
  try {
    await action
    notifications.show({ message })
  } catch (e) {
    notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
  }
}

function countLabel(vault: Vault): string {
  return `${vault.category_count ?? 0} categories, ${vault.bookmark_count ?? 0} bookmarks`
}

function VaultRow({ vault, globalEnabled }: { vault: Vault; globalEnabled: boolean }): ReactElement {
  const [passphrase, setPassphrase] = useState('')
  const [name, setName] = useState(vault.name)
  const [recovering, setRecovering] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null)
  const [confirmNewKey, setConfirmNewKey] = useState(false)
  const unlock = useUnlockVault()
  const lock = useLockVault()
  const rename = useRenameVault()
  const newKey = useNewRecoveryKey()
  const join = useJoinGlobal()
  const leave = useLeaveGlobal()

  // A locked vault's key isn't available to seal, but leaving needs no key.
  const canJoin = vault.unlocked || vault.in_global

  async function onUnlock(): Promise<void> {
    await report(unlock.mutateAsync({ id: vault.id, passphrase }), `${vault.name} unlocked`)
    setPassphrase('')
  }

  const [keyPassphrase, setKeyPassphrase] = useState('')

  async function onNewKey(): Promise<void> {
    try {
      const res = await newKey.mutateAsync({ id: vault.id, passphrase: keyPassphrase })
      setConfirmNewKey(false)
      setKeyPassphrase('')
      setRecoveryKey(res.recovery_key)
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack gap="sm">
        <Group justify="space-between">
          <Text fw={500}>{vault.name}</Text>
          <Badge color={vault.unlocked ? 'green' : 'gray'} variant="light">
            {vault.unlocked ? 'Unlocked' : 'Locked'}
          </Badge>
        </Group>

        {vault.unlocked ? (
          <>
            <Text size="sm" c="dimmed">
              {countLabel(vault)}
            </Text>
            <Group align="flex-end">
              <TextInput
                label="Name"
                value={name}
                onChange={(e) => setName(e.currentTarget.value)}
                style={{ flex: 1 }}
              />
              <Button
                variant="default"
                disabled={!name.trim() || name === vault.name}
                loading={rename.isPending}
                onClick={() => void report(rename.mutateAsync({ id: vault.id, name }), 'Vault renamed')}
              >
                Rename
              </Button>
            </Group>
            <Group>
              <Button
                variant="light"
                leftSection={<IconLock {...iconSm} />}
                loading={lock.isPending}
                onClick={() => void report(lock.mutateAsync(vault.id), `${vault.name} locked`)}
              >
                Lock
              </Button>
              <Button variant="default" loading={newKey.isPending} onClick={() => setConfirmNewKey(true)}>
                {vault.has_recovery_key ? 'Replace recovery key' : 'Create recovery key'}
              </Button>
            </Group>
          </>
        ) : (
          <Group align="flex-end">
            <PasswordInput
              label={`Passphrase for ${vault.name}`}
              value={passphrase}
              onChange={(e) => setPassphrase(e.currentTarget.value)}
              style={{ flex: 1 }}
            />
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              disabled={passphrase.length < 8}
              loading={unlock.isPending}
              onClick={() => void onUnlock()}
            >
              Unlock
            </Button>
          </Group>
        )}

        {globalEnabled ? (
          <Switch
            label="Opens with the global passphrase"
            description={canJoin ? undefined : 'Unlock this vault to turn this on.'}
            checked={vault.in_global}
            disabled={!canJoin || join.isPending || leave.isPending}
            onChange={(e) => {
              if (e.currentTarget.checked) {
                void report(join.mutateAsync(vault.id), 'Added to global unlock')
              } else {
                void report(leave.mutateAsync(vault.id), 'Removed from global unlock')
              }
            }}
          />
        ) : null}

        <Group justify="space-between">
          {vault.has_recovery_key ? (
            <Button variant="subtle" size="compact-sm" onClick={() => setRecovering(true)}>
              Forgot passphrase
            </Button>
          ) : (
            <span />
          )}
          <Button variant="subtle" color="red" size="compact-sm" onClick={() => setDeleting(true)}>
            Delete vault
          </Button>
        </Group>
      </Stack>

      {recovering ? <RecoverVaultModal vault={vault} onClose={() => setRecovering(false)} /> : null}
      {deleting ? <DeleteVaultModal vault={vault} onClose={() => setDeleting(false)} /> : null}
      <Modal
        opened={confirmNewKey}
        onClose={() => {
          setConfirmNewKey(false)
          setKeyPassphrase('')
        }}
        title={vault.has_recovery_key ? 'Replace recovery key?' : 'Create recovery key'}
        centered
      >
        <Stack>
          {vault.has_recovery_key ? (
            <Text size="sm">
              The current recovery key for {vault.name} stops working at once. Only the new key, shown next, can recover
              this vault.
            </Text>
          ) : null}
          <PasswordInput
            label={`Passphrase for ${vault.name}`}
            value={keyPassphrase}
            onChange={(e) => setKeyPassphrase(e.currentTarget.value)}
            data-autofocus
          />
          <Group justify="flex-end">
            <Button
              color={vault.has_recovery_key ? 'red' : undefined}
              disabled={!keyPassphrase}
              loading={newKey.isPending}
              onClick={() => void onNewKey()}
            >
              {vault.has_recovery_key ? 'Replace key' : 'Create key'}
            </Button>
          </Group>
        </Stack>
      </Modal>
      {recoveryKey ? <RecoveryKeyModal recoveryKey={recoveryKey} onDone={() => setRecoveryKey(null)} /> : null}
    </Paper>
  )
}

export function VaultList(): ReactElement {
  const list = useVaults()
  const global = useGlobalPassphrase()
  if (list.isError) return <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} />
  if (list.isLoading) return <Skeleton height={120} />
  const vaults = list.data?.vaults ?? []
  if (vaults.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No vaults yet.
      </Text>
    )
  }
  return (
    <Stack>
      {vaults.map((vault) => (
        <VaultRow key={vault.id} vault={vault} globalEnabled={!!global.data?.enabled} />
      ))}
    </Stack>
  )
}
