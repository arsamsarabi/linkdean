import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
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
import { api } from '../api'
import { useUi } from '../store'

const iconSm = { size: 16, stroke: 1.5 } as const

export function VaultPanel() {
  const qc = useQueryClient()
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  const [pass, setPass] = useState('')
  const [busy, setBusy] = useState(false)
  const status = useQuery({ queryKey: ['vault'], queryFn: api.vaultStatus })

  async function setup() {
    setBusy(true)
    try {
      await api.vaultSetup(pass)
      setVaultUnlocked(true)
      setPass('')
      qc.invalidateQueries({ queryKey: ['vault'] })
      qc.invalidateQueries({ queryKey: ['me'] })
      notifications.show({ message: 'Vault ready + unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  async function unlock() {
    setBusy(true)
    try {
      await api.vaultUnlock(pass)
      setVaultUnlocked(true)
      setPass('')
      qc.invalidateQueries({ queryKey: ['vault'] })
      qc.invalidateQueries({ queryKey: ['me'] })
      qc.invalidateQueries({ queryKey: ['categories'] })
      notifications.show({ message: 'Vault unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  async function lock() {
    setBusy(true)
    try {
      await api.vaultLock()
      setVaultUnlocked(false)
      qc.invalidateQueries({ queryKey: ['vault'] })
      qc.invalidateQueries({ queryKey: ['me'] })
      qc.invalidateQueries({ queryKey: ['categories'] })
      qc.invalidateQueries({ queryKey: ['bookmarks'] })
      notifications.show({ message: 'Vault locked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  return (
    <Stack gap="lg" maw={480}>
      <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
        Password-protected categories. AES-GCM at rest. Not in Meilisearch until unlocked
        (then Postgres search).
      </Alert>

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
              onClick={setup}
              disabled={pass.length < 8}
              loading={busy}
            >
              Setup vault
            </Button>
          ) : (
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              onClick={unlock}
              disabled={pass.length < 8 || !!status.data?.unlocked}
              loading={busy}
            >
              Unlock
            </Button>
          )}
          <Button
            variant="light"
            leftSection={<IconLock {...iconSm} />}
            onClick={lock}
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
