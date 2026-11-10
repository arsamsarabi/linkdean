import { useState, type ReactElement } from 'react'
import { Badge, Group, Paper, SegmentedControl, Skeleton, Stack, Text } from '@mantine/core'
import { IconHistory } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useActivity } from '@/hooks'
import type { ActivityAction, ActivitySubjectType } from '@/schemas'
import { LibraryHealthHeader } from './LibraryHealthHeader'

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

export function HistoryPage(): ReactElement {
  const [subject, setSubject] = useState<SubjectFilter>('all')
  const [action, setAction] = useState<ActionFilter>('all')
  const list = useActivity(true, { subjectType: subject, action })

  // Skeleton only on true cold start (no cache / no placeholder). Prefetch + keepPreviousData
  // cover hover-open and filter hops — no delayed timer needed.
  const showSkeleton = list.isPending
  const swappingFilters = list.isFetching && list.isPlaceholderData

  return (
    <Stack gap="xl" maw={900}>
      <LibraryHealthHeader
        title="History"
        description="Your own create, edit and delete log."
        icon={IconHistory}
        color="violet"
      />
      <Stack gap="md">
        <Group gap="sm" wrap="wrap">
          <SegmentedControl
            size="xs"
            value={subject}
            onChange={(v) => setSubject(v as SubjectFilter)}
            aria-label="Filter by type"
            data={[
              { label: 'All', value: 'all' },
              { label: 'Bookmarks', value: 'bookmark' },
              { label: 'Lists', value: 'category' },
              { label: 'Boards', value: 'dashboard' },
            ]}
          />
          <SegmentedControl
            size="xs"
            value={action}
            onChange={(v) => setAction(v as ActionFilter)}
            aria-label="Filter by action"
            data={[
              { label: 'All', value: 'all' },
              { label: 'Created', value: 'created' },
              { label: 'Updated', value: 'updated' },
              { label: 'Deleted', value: 'deleted' },
              { label: 'Restored', value: 'restored' },
            ]}
          />
        </Group>

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
            <Paper key={e.id} withBorder p="sm">
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
            </Paper>
          ))}
        </Stack>
      </Stack>
    </Stack>
  )
}
