import { useEffect, type ReactElement } from 'react'
import { Button, Group, Select } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLock } from '@tabler/icons-react'
import { useLockAllVaults, useLockVault, useVaults } from '@/hooks'
import { useLibraryPageModel } from './useLibraryPageModel'

const iconSm = { size: 16, stroke: 1.5 } as const

/** Vault filter and lock controls for the Vaults page. Locked vaults never appear here. */
export function VaultsBar(): ReactElement {
  const lib = useLibraryPageModel()
  const vaults = useVaults()
  const lock = useLockVault()
  const lockAll = useLockAllVaults()
  const open = (vaults.data?.vaults ?? []).filter((v) => v.unlocked)
  const openKey = open.map((v) => v.id).join()
  const { vaultId, setLibrarySearch, exitSelectMode } = lib

  useEffect(() => {
    if (!vaults.data) return
    exitSelectMode()
    if (vaultId && !openKey.split(',').includes(vaultId)) setLibrarySearch({ vaultId: null })
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- only when the set of open vaults changes
  }, [openKey])

  function run(action: Promise<unknown>, message: string): void {
    action.then(
      () => notifications.show({ message }),
      (e: unknown) => notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    )
  }

  return (
    <Group justify="space-between" align="flex-end" wrap="wrap" gap="sm">
      <Select
        label="Vault"
        placeholder="All open vaults"
        clearable
        data={open.map((v) => ({ value: v.id, label: v.name }))}
        value={vaultId}
        onChange={(next) => setLibrarySearch({ vaultId: next })}
      />
      <Group gap="xs" wrap="wrap">
        {open.map((v) => (
          <Button
            key={v.id}
            variant="default"
            leftSection={<IconLock {...iconSm} />}
            aria-label={`Lock ${v.name}`}
            loading={lock.isPending && lock.variables === v.id}
            onClick={() => run(lock.mutateAsync(v.id), `${v.name} locked`)}
          >
            {v.name}
          </Button>
        ))}
        <Button
          variant="light"
          leftSection={<IconLock {...iconSm} />}
          loading={lockAll.isPending}
          onClick={() => run(lockAll.mutateAsync(), 'All vaults locked')}
        >
          Lock all
        </Button>
      </Group>
    </Group>
  )
}
