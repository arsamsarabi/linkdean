import { useState, type ReactElement } from 'react'
import { Button, Checkbox, Divider, Group, Paper, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useBans, useLiftBans } from '@/hooks'
import type { BanView } from '@/schemas'
import { banEndLabel } from '@/services'
import { BanEditModal } from './BanEditModal'

function label(ban: BanView): string {
  return ban.display_name?.trim() || ban.email
}

/** Active bans with bulk actions. Hidden while nobody is banned. */
export function BannedUsersPanel(): ReactElement | null {
  const bans = useBans()
  const lift = useLiftBans()
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set())
  const [editing, setEditing] = useState<'reason' | 'duration' | null>(null)
  const rows = bans.data?.bans ?? []
  const selectedIds = rows.filter((b) => selected.has(b.user_id)).map((b) => b.user_id)

  function toggle(id: string): void {
    setSelected((prev) => {
      const next = new Set(prev)
      if (!next.delete(id)) next.add(id)
      return next
    })
  }

  async function unban(): Promise<void> {
    try {
      await lift.mutateAsync(selectedIds)
      notifications.show({ message: `${selectedIds.length} unbanned` })
      setSelected(new Set())
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  if (rows.length === 0) return null

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <div>
          <Text fw={500}>Banned users</Text>
          <Text size="sm" c="dimmed">
            Select accounts to unban them or change their bans.
          </Text>
        </div>
        {rows.map((ban, i) => (
          <Stack key={ban.user_id} gap="xs">
            <Group wrap="nowrap" align="flex-start" gap="sm">
              <Checkbox
                mt={2}
                aria-label={`Select ${label(ban)}`}
                checked={selected.has(ban.user_id)}
                onChange={() => toggle(ban.user_id)}
              />
              <Stack gap={2} miw={0} flex={1}>
                <Text size="sm" fw={500} truncate>
                  {label(ban)}
                </Text>
                <Text size="xs" c="dimmed" truncate>
                  {ban.email} · banned {banEndLabel(ban.ends_at)}
                </Text>
                <Text size="sm">{ban.reason}</Text>
              </Stack>
            </Group>
            {i < rows.length - 1 ? <Divider /> : null}
          </Stack>
        ))}
        {selectedIds.length > 0 ? (
          <Group gap="xs" wrap="wrap">
            <Button size="xs" loading={lift.isPending} onClick={() => void unban()}>
              Unban {selectedIds.length}
            </Button>
            <Button size="xs" variant="default" onClick={() => setEditing('duration')}>
              Change duration
            </Button>
            <Button size="xs" variant="default" onClick={() => setEditing('reason')}>
              Change reason
            </Button>
          </Group>
        ) : null}
      </Stack>
      {editing ? (
        <BanEditModal
          mode={editing}
          userIds={selectedIds}
          onClose={() => setEditing(null)}
          onDone={() => {
            setEditing(null)
            setSelected(new Set())
          }}
        />
      ) : null}
    </Paper>
  )
}
