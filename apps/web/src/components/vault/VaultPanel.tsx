import { useState, type ReactElement } from 'react'
import {
  Alert,
  Badge,
  Button,
  Group,
  Paper,
  PasswordInput,
  Skeleton,
  Stack,
  Text,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconInfoCircle,
  IconLock,
  IconLockOpen,
  IconLockPlus,
} from '@tabler/icons-react'
import {
  useVaultLock,
  useVaultSetup,
  useVaultStatus,
  useVaultUnlock,
} from '../../hooks/vault/useVault'
import { QueryErrorAlert } from '../query-error-alert'

const iconSm = { size: 16, stroke: 1.5 } as const

export function VaultPanel(): ReactElement {
  const [pass, setPass] = useState('')
  const status = useVaultStatus()
  const setupMut = useVaultSetup()
  const unlockMut = useVaultUnlock()
  const lockMut = useVaultLock()
  const busy = setupMut.isPending || unlockMut.isPending || lockMut.isPending

  async function setup(): Promise<void> {
    try {
      await setupMut.mutateAsync(pass)
      setPass('')
      notifications.show({ message: 'Vault ready + unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function unlock(): Promise<void> {
    try {
      await unlockMut.mutateAsync(pass)
      setPass('')
      notifications.show({ message: 'Vault unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function lock(): Promise<void> {
    try {
      await lockMut.mutateAsync()
      notifications.show({ message: 'Vault locked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Stack gap="lg" maw={480}>
      <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
        Password-protected categories. AES-GCM at rest. Not in Meilisearch until unlocked
        (then Postgres search).
      </Alert>

      {status.isError ? (
        <QueryErrorAlert error={status.error} onRetry={() => void status.refetch()} />
      ) : null}

      <Paper p="xl" withBorder pos="relative">
        <Stack gap="lg">
          {status.isLoading ? (
            <Skeleton height={28} width={220} />
          ) : (
            <Group gap="sm">
              <Text size="sm" fw={500}>
                Status
              </Text>
              <Badge color={status.data?.configured ? 'green' : 'gray'} variant="light">
                {status.data?.configured ? 'Configured' : 'Not set up'}
              </Badge>
              <Badge color={status.data?.unlocked ? 'green' : 'gray'} variant="outline">
                {status.data?.unlocked ? 'Unlocked' : 'Locked'}
              </Badge>
            </Group>
          )}

          <PasswordInput
            label="Passphrase"
            value={pass}
            onChange={(e) => setPass(e.currentTarget.value)}
          />

          {!status.data?.configured ? (
            <Button
              leftSection={<IconLockPlus {...iconSm} />}
              onClick={() => void setup()}
              disabled={pass.length < 8}
              loading={busy}
            >
              Setup vault
            </Button>
          ) : (
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              onClick={() => void unlock()}
              disabled={pass.length < 8 || !!status.data?.unlocked}
              loading={busy}
            >
              Unlock
            </Button>
          )}
          <Button
            variant="light"
            leftSection={<IconLock {...iconSm} />}
            onClick={() => void lock()}
            disabled={!status.data?.unlocked}
            loading={busy}
          >
            Lock
          </Button>
        </Stack>
      </Paper>
    </Stack>
  )
}
