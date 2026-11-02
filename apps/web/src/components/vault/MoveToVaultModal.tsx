import { useState, type ReactElement } from 'react'
import { Alert, Anchor, Button, List, Modal, Select, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { Link } from '@tanstack/react-router'
import { z } from 'zod'
import { ApiError } from '@/api/client'
import { useMoveToVault, useVaults } from '@/hooks'
import { useChildDialog } from '@/hooks/render'
import { vaultMoveBlockerSchema, type VaultMoveBlocker } from '@/schemas'

const OUT_OF_VAULT = 'out'
const blockedBodySchema = z.object({ blockers: z.array(vaultMoveBlockerSchema) })

type Props = {
  categoryIds: string[]
  bookmarkIds: string[]
  onClose: () => void
}

function fixTarget(blocker: VaultMoveBlocker): string {
  return blocker.kind === 'dashboard' ? '/' : '/library'
}

export function MoveToVaultModal({ categoryIds, bookmarkIds, onClose }: Props): ReactElement {
  const vaults = useVaults()
  const move = useMoveToVault()
  const [destination, setDestination] = useState<string | null>(null)
  const [blockers, setBlockers] = useState<VaultMoveBlocker[]>([])

  const options = [
    ...(vaults.data?.vaults ?? []).filter((v) => v.unlocked).map((v) => ({ value: v.id, label: v.name })),
    { value: OUT_OF_VAULT, label: 'Out of vault' },
  ]

  async function onMove(): Promise<void> {
    setBlockers([])
    try {
      await move.mutateAsync({
        category_ids: categoryIds,
        bookmark_ids: bookmarkIds,
        destination_vault_id: destination === OUT_OF_VAULT ? null : destination,
      })
      notifications.show({ message: destination === OUT_OF_VAULT ? 'Moved to Library' : 'Moved' })
      onClose()
    } catch (e) {
      const parsed = e instanceof ApiError && e.status === 409 ? blockedBodySchema.safeParse(e.body) : null
      if (parsed?.success) {
        setBlockers(parsed.data.blockers)
        return
      }
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  useChildDialog(true)
  return (
    <Modal opened onClose={onClose} title="Move to vault">
      <Stack>
        <Select label="Destination" data={options} value={destination} onChange={setDestination} />
        <Text size="xs" c="dimmed">
          Locked vaults aren't listed. Unlock one first to move items into it.
        </Text>
        {blockers.length > 0 ? (
          <Alert color="yellow" variant="light" title="Fix these first, then move again">
            <List size="sm" spacing="xs">
              {blockers.map((blocker) => (
                <List.Item key={`${blocker.kind}:${blocker.subject_id}:${blocker.related_id ?? ''}`}>
                  <Anchor component={Link} to={fixTarget(blocker)} onClick={onClose}>
                    {blocker.message}
                  </Anchor>
                </List.Item>
              ))}
            </List>
          </Alert>
        ) : null}
        <Button disabled={!destination} loading={move.isPending} onClick={() => void onMove()}>
          Move
        </Button>
      </Stack>
    </Modal>
  )
}
