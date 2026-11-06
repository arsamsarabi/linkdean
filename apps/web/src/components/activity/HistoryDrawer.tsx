import { useState, type ReactElement } from 'react'
import { ActionIcon, Badge, Box, Drawer, Group, SegmentedControl, Skeleton, Stack, Text } from '@mantine/core'
import { IconHistory } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useActivity, usePrefetchActivity } from '@/hooks'
import { useDrawerSize } from '@/hooks/render'
import type { ActivityAction, ActivitySubjectType } from '@/schemas'

type Props = {
  opened: boolean
  onClose: () => void
}

function actionLabel(action: ActivityAction): string {
  switch (action) {
    case 'created':
      return 'Created'
    case 'updated':
      return 'Updated'
    case 'deleted':
      return 'Deleted'
    case 'restored':
      return 'Restored'
  }
}

function actionColor(action: ActivityAction): string {
  switch (action) {
    case 'created':
      return 'teal'
    case 'updated':
      return 'blue'
    case 'deleted':
      return 'red'
    case 'restored':
      return 'grape'
  }
}

type SubjectFilter = 'all' | ActivitySubjectType
type ActionFilter = 'all' | ActivityAction

export function HistoryDrawer({ opened, onClose }: Props): ReactElement {
  const drawerSize = useDrawerSize()
  const [subject, setSubject] = useState<SubjectFilter>('all')
  const [action, setAction] = useState<ActionFilter>('all')
  const list = useActivity(opened, { subjectType: subject, action })

  // Skeleton only on true cold start (no cache / no placeholder). Prefetch + keepPreviousData
  // cover hover-open and filter hops — no delayed timer needed.
  const showSkeleton = list.isPending
  const swappingFilters = list.isFetching && list.isPlaceholderData

  return (
    <Drawer opened={opened} onClose={onClose} position="right" size={drawerSize} title="History" padding="md">
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          Personal create / edit / delete log
        </Text>

        <SegmentedControl
          size="xs"
          fullWidth
          value={subject}
          onChange={(v) => setSubject(v as SubjectFilter)}
          data={[
            { label: 'All', value: 'all' },
            { label: 'Bookmarks', value: 'bookmark' },
            { label: 'Lists', value: 'category' },
            { label: 'Boards', value: 'dashboard' },
          ]}
        />

        <SegmentedControl
          size="xs"
          fullWidth
          value={action}
          onChange={(v) => setAction(v as ActionFilter)}
          data={[
            { label: 'All', value: 'all' },
            { label: 'Created', value: 'created' },
            { label: 'Updated', value: 'updated' },
            { label: 'Deleted', value: 'deleted' },
            { label: 'Restored', value: 'restored' },
          ]}
        />

        {list.isError && !list.data ? <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} /> : null}

        {showSkeleton ? (
          <Stack gap="sm">
            <Skeleton height={56} />
            <Skeleton height={56} />
            <Skeleton height={56} />
          </Stack>
        ) : null}

        {!showSkeleton && list.isSuccess && list.data.events.length === 0 ? (
          <Text size="sm" c="dimmed">
            No activity yet.
          </Text>
        ) : null}

        <Stack
          gap="xs"
          style={{
            opacity: swappingFilters ? 0.55 : 1,
            transition: 'opacity 120ms ease',
          }}
        >
          {(list.data?.events ?? []).map((e) => (
            <Box
              key={e.id}
              p="sm"
              style={{
                borderRadius: 8,
                border: '1px solid var(--mantine-color-default-border)',
              }}
            >
              <Stack gap={4}>
                <Group gap="xs" justify="space-between" wrap="nowrap">
                  <Group gap={6} wrap="nowrap">
                    <Badge size="xs" variant="light" color={actionColor(e.action)}>
                      {actionLabel(e.action)}
                    </Badge>
                    <Badge size="xs" variant="outline" color="gray">
                      {e.subject_type}
                    </Badge>
                  </Group>
                  <Text size="xs" c="dimmed">
                    {new Date(e.created_at).toLocaleString()}
                  </Text>
                </Group>
                <Text size="sm" fw={500}>
                  {e.summary}
                </Text>
              </Stack>
            </Box>
          ))}
        </Stack>
      </Stack>
    </Drawer>
  )
}

export function HistoryButton({ onClick }: { onClick: () => void }): ReactElement {
  const prefetch = usePrefetchActivity()
  return (
    <ActionIcon
      variant="subtle"
      size="lg"
      radius="md"
      aria-label="History"
      onClick={onClick}
      onMouseEnter={prefetch}
      onFocus={prefetch}
      onPointerDown={prefetch}
    >
      <IconHistory size={20} stroke={1.5} />
    </ActionIcon>
  )
}
