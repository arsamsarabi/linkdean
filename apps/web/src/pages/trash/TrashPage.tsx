import { useMemo, useState, type ReactElement } from 'react'
import { Button, Checkbox, Group, Stack, Table, Text, ThemeIcon, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconRestore, IconTrash } from '@tabler/icons-react'
import { ConfirmModal, EmptyState, MASCOT, QueryErrorAlert } from '@/components'
import { useMe, usePurgeTrashItem, usePurgeTrashSelected, useRestoreTrashItem, useTrash } from '@/hooks'
import type { TrashItem, TrashKind } from '@/schemas'

function kindLabel(kind: TrashKind): string {
  switch (kind) {
    case 'bookmark':
      return 'Bookmark'
    case 'category':
      return 'Folder'
    case 'dashboard':
      return 'Dashboard'
  }
}

function itemKey(item: TrashItem): string {
  return `${item.kind}:${item.id}`
}

export function TrashPage(): ReactElement {
  const me = useMe()
  const trash = useTrash()
  const restore = useRestoreTrashItem()
  const purgeOne = usePurgeTrashItem()
  const purgeMany = usePurgeTrashSelected()
  const [selected, setSelected] = useState<Set<string>>(() => new Set())
  const [purgeConfirm, setPurgeConfirm] = useState<TrashItem[] | null>(null)

  const retention = me.data?.settings.trash_retention_days ?? 30
  const items = useMemo(() => trash.data || [], [trash.data])
  const allKeys = useMemo(() => items.map(itemKey), [items])
  const allSelected = allKeys.length > 0 && allKeys.every((k) => selected.has(k))

  function toggle(key: string): void {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  async function onRestore(item: TrashItem): Promise<void> {
    try {
      await restore.mutateAsync({ kind: item.kind, id: item.id })
      notifications.show({ message: `Restored “${item.title || kindLabel(item.kind)}”` })
      setSelected((prev) => {
        const next = new Set(prev)
        next.delete(itemKey(item))
        return next
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function confirmPurge(): Promise<void> {
    if (!purgeConfirm || purgeConfirm.length === 0) return
    try {
      if (purgeConfirm.length === 1) {
        const item = purgeConfirm[0]!
        await purgeOne.mutateAsync({ kind: item.kind, id: item.id })
        notifications.show({ message: 'Permanently deleted' })
      } else {
        const result = await purgeMany.mutateAsync(purgeConfirm.map((i) => ({ kind: i.kind, id: i.id })))
        notifications.show({
          message: `Purged ${result.purged ?? purgeConfirm.length}${result.failed ? `, ${result.failed} failed` : ''}`,
        })
      }
      setSelected(new Set())
      setPurgeConfirm(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const selectedItems = items.filter((i) => selected.has(itemKey(i)))
  const pending = restore.isPending || purgeOne.isPending || purgeMany.isPending

  return (
    <Stack gap="xl">
      <Group gap="md" justify="space-between" wrap="wrap">
        <Group gap="md">
          <ThemeIcon size={40} radius="md" variant="light" color="red">
            <IconTrash size={22} stroke={1.5} />
          </ThemeIcon>
          <div>
            <Title order={2}>Deleted items</Title>
            <Text size="sm" c="dimmed">
              Soft-deleted items stay for {retention} days, then purge automatically.
            </Text>
          </div>
        </Group>
        <Group gap="xs">
          <Button
            variant="default"
            size="compact-sm"
            disabled={selectedItems.length === 0 || pending}
            onClick={() => setSelected(new Set())}
          >
            Clear all
          </Button>
          <Button
            color="red"
            size="compact-sm"
            disabled={selectedItems.length === 0 || pending}
            onClick={() => setPurgeConfirm(selectedItems)}
          >
            Delete forever
          </Button>
        </Group>
      </Group>

      {trash.isError ? (
        <QueryErrorAlert error={trash.error} onRetry={() => void trash.refetch()} title="Trash failed" />
      ) : null}

      {!trash.isLoading && items.length === 0 ? (
        <EmptyState
          illustration={{ src: MASCOT.smiling }}
          title="Nothing in Deleted"
          description="Deleted bookmarks, folders, and dashboards land here."
        />
      ) : (
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th w={40}>
                <Checkbox
                  aria-label="Select all"
                  checked={allSelected}
                  indeterminate={selected.size > 0 && !allSelected}
                  onChange={() => setSelected(allSelected ? new Set() : new Set(allKeys))}
                />
              </Table.Th>
              <Table.Th>Type</Table.Th>
              <Table.Th>Title</Table.Th>
              <Table.Th>Deleted</Table.Th>
              <Table.Th w={200} />
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {items.map((item) => {
              const key = itemKey(item)
              return (
                <Table.Tr key={key}>
                  <Table.Td>
                    <Checkbox
                      aria-label={`Select ${item.title}`}
                      checked={selected.has(key)}
                      onChange={() => toggle(key)}
                    />
                  </Table.Td>
                  <Table.Td>
                    <Text size="sm">{kindLabel(item.kind)}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="sm" fw={500} lineClamp={1}>
                      {item.title || 'Untitled'}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="sm" c="dimmed">
                      {new Date(item.deleted_at).toLocaleString()}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Group gap="xs" justify="flex-end">
                      <Button
                        size="compact-xs"
                        variant="light"
                        leftSection={<IconRestore size={14} stroke={1.5} />}
                        loading={restore.isPending}
                        onClick={() => void onRestore(item)}
                      >
                        Restore
                      </Button>
                      <Button size="compact-xs" color="red" variant="subtle" onClick={() => setPurgeConfirm([item])}>
                        Delete forever
                      </Button>
                    </Group>
                  </Table.Td>
                </Table.Tr>
              )
            })}
          </Table.Tbody>
        </Table>
      )}

      <ConfirmModal
        opened={!!purgeConfirm}
        onClose={() => setPurgeConfirm(null)}
        onConfirm={() => void confirmPurge()}
        title="Delete forever?"
        body={
          purgeConfirm && purgeConfirm.length > 1
            ? `Permanently delete ${purgeConfirm.length} items? This cannot be undone.`
            : 'Permanently delete this item? This cannot be undone.'
        }
        confirmLabel="Delete forever"
        loading={purgeOne.isPending || purgeMany.isPending}
      />
    </Stack>
  )
}
