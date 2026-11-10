import { useState, type ReactElement } from 'react'
import { Box, Button, Group, LoadingOverlay, Paper, Radio, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCopy } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useDuplicates, useMergeDuplicateGroup, useSyncDuringRender } from '@/hooks'
import type { Bookmark, DuplicateGroup } from '@/schemas'
import { pickDefaultKeepId } from '@/services'
import { LibraryHealthHeader } from './LibraryHealthHeader'

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
    <Paper withBorder p="md">
      <Stack gap="sm">
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
    </Paper>
  )
}

export function DuplicatesPage(): ReactElement {
  const dupes = useDuplicates(true)
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
    <Stack gap="xl" maw={900}>
      <LibraryHealthHeader
        title="Duplicates"
        description="Bookmarks saved more than once. Merge each set into one."
        icon={IconCopy}
        color="orange"
      />
      <Box pos="relative" mih={120}>
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
    </Stack>
  )
}
