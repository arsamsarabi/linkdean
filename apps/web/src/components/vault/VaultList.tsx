import { useState, type ReactElement } from 'react'
import { Badge, Button, Group, Paper, PasswordInput, Skeleton, Stack, Switch, Text, TextInput } from '@mantine/core'
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
  const [globalPass, setGlobalPass] = useState('')
  const [recovering, setRecovering] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null)
  const unlock = useUnlockVault()
  const lock = useLockVault()
  const rename = useRenameVault()
  const newKey = useNewRecoveryKey()
  const join = useJoinGlobal()
  const leave = useLeaveGlobal()

  async function onUnlock(): Promise<void> {
    await report(unlock.mutateAsync({ id: vault.id, passphrase }), `${vault.name} unlocked`)
    setPassphrase('')
  }

  async function onNewKey(): Promise<void> {
    try {
      const res = await newKey.mutateAsync(vault.id)
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
            {globalEnabled && !vault.in_global ? (
              <Group align="flex-end">
                <PasswordInput
                  label="Global passphrase"
                  description="Lets the global passphrase open this vault too."
                  value={globalPass}
                  onChange={(e) => setGlobalPass(e.currentTarget.value)}
                  style={{ flex: 1 }}
                />
                <Button
                  variant="default"
                  disabled={globalPass.length < 8}
                  loading={join.isPending}
                  onClick={() => {
                    void report(join.mutateAsync({ id: vault.id, passphrase: globalPass }), 'Added to global unlock')
                    setGlobalPass('')
                  }}
                >
                  Add to global unlock
                </Button>
              </Group>
            ) : null}
            <Group>
              <Button
                variant="light"
                leftSection={<IconLock {...iconSm} />}
                loading={lock.isPending}
                onClick={() => void report(lock.mutateAsync(vault.id), `${vault.name} locked`)}
              >
                Lock
              </Button>
              <Button variant="default" loading={newKey.isPending} onClick={() => void onNewKey()}>
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

        {globalEnabled && vault.in_global ? (
          <Switch
            label="Opens with the global passphrase"
            checked
            onChange={() => void report(leave.mutateAsync(vault.id), 'Removed from global unlock')}
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
