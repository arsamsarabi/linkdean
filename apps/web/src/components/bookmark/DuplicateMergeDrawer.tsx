import { useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Box,
  Button,
  Drawer,
  Group,
  LoadingOverlay,
  Radio,
  Stack,
  Text,
  Title,
  Tooltip,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCopy } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useDuplicates, useMergeDuplicateGroup, useSyncDuringRender } from '@/hooks'
import type { Bookmark, DuplicateGroup } from '@/schemas'
import { pickDefaultKeepId } from '@/services'

type Props = {
  opened: boolean
  onClose: () => void
}

function GroupCard({
  group,
  merging,
  onMerge,
}: {
  group: DuplicateGroup
  merging: boolean
  onMerge: (keep: Bookmark, losers: Bookmark[]) => Promise<void>
}): ReactElement {
  const [keepId, setKeepId] = useState(() => pickDefaultKeepId(group.bookmarks) ?? '')

  useSyncDuringRender([group.url, group.bookmarks], () => {
    setKeepId(pickDefaultKeepId(group.bookmarks) ?? '')
  })

  return (
    <Stack gap="sm" p="md" style={{ border: '1px solid var(--mantine-color-default-border)', borderRadius: 8 }}>
      <Text size="sm" fw={600} lineClamp={2}>
        {group.url}
      </Text>
      <Text size="xs" c="dimmed">
        {group.bookmarks.length} copies · keep one, merge tags/categories, delete the rest
      </Text>
      <Radio.Group value={keepId} onChange={setKeepId}>
        <Stack gap="xs">
          {group.bookmarks.map((b) => (
            <Radio
              key={b.id}
              value={b.id}
              label={
                <Stack gap={2}>
                  <Text size="sm">{b.title || b.url}</Text>
                  <Text size="xs" c="dimmed">
                    updated {new Date(b.updated_at).toLocaleString()}
                    {b.tags.length ? ` · ${b.tags.map((t) => `#${t}`).join(' ')}` : ''}
                  </Text>
                </Stack>
              }
            />
          ))}
        </Stack>
      </Radio.Group>
      <Group justify="flex-end">
        <Button
          size="compact-sm"
          loading={merging}
          disabled={!keepId}
          onClick={() => {
            const keep = group.bookmarks.find((b) => b.id === keepId)
            if (!keep) return
            const losers = group.bookmarks.filter((b) => b.id !== keepId)
            void onMerge(keep, losers)
          }}
        >
          Merge into selected
        </Button>
      </Group>
    </Stack>
  )
}

export function DuplicatesButton({ onClick }: { onClick: () => void }): ReactElement {
  return (
    <Tooltip label="Find and merge duplicate URLs" position="bottom">
      <ActionIcon variant="subtle" size="lg" radius="md" aria-label="Find and merge duplicate URLs" onClick={onClick}>
        <IconCopy size={20} stroke={1.5} />
      </ActionIcon>
    </Tooltip>
  )
}

export function DuplicateMergeDrawer({ opened, onClose }: Props): ReactElement {
  const dupes = useDuplicates(opened)
  const merge = useMergeDuplicateGroup()
  const [busyUrl, setBusyUrl] = useState<string | null>(null)

  async function onMerge(keep: Bookmark, losers: Bookmark[]): Promise<void> {
    setBusyUrl(keep.url)
    try {
      await merge.mutateAsync({ keep, losers })
      notifications.show({ message: `Merged ${losers.length + 1} copies → 1` })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusyUrl(null)
    }
  }

  const groups = dupes.data?.groups ?? []

  return (
    <Drawer opened={opened} onClose={onClose} title="Duplicate URLs" position="right" size="md" padding="lg">
      <Box pos="relative">
        <LoadingOverlay visible={dupes.isLoading} zIndex={100} overlayProps={{ blur: 1 }} />
        <Stack gap="md">
          {dupes.isError ? (
            <QueryErrorAlert error={dupes.error} onRetry={() => void dupes.refetch()} title="Duplicates failed" />
          ) : null}
          {!dupes.isLoading && !dupes.isError && groups.length === 0 ? (
            <Text size="sm" c="dimmed">
              No duplicate URLs in your library (vault excluded).
            </Text>
          ) : null}
          {groups.length > 0 ? (
            <>
              <Title order={5}>
                {groups.length} duplicate group{groups.length === 1 ? '' : 's'}
              </Title>
              {groups.map((g) => (
                <GroupCard key={g.url} group={g} merging={busyUrl === g.url && merge.isPending} onMerge={onMerge} />
              ))}
            </>
          ) : null}
        </Stack>
      </Box>
    </Drawer>
  )
}
