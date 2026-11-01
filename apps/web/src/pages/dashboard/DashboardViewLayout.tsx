import { type ReactElement } from 'react'
import { SimpleGrid, Stack, Text, Title } from '@mantine/core'
import { IconLayoutDashboard, IconPencil } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { BookmarkCard, EmptyState } from '@/components'
import type { Dashboard } from '@/schemas'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  view: Dashboard
  empty: boolean
  canEditLayout: boolean
  viewerUserId: string | undefined
  onStartEdit: () => void
}

export function DashboardViewLayout({ view, empty, canEditLayout, viewerUserId, onStartEdit }: Props): ReactElement {
  if (empty) {
    return (
      <EmptyState
        icon={<IconLayoutDashboard size={28} stroke={1.5} />}
        title="Dashboard is empty"
        description={
          canEditLayout
            ? 'Pin frequently used links into sections for a quick glance home.'
            : 'This shared dashboard has no sections yet.'
        }
        action={
          canEditLayout
            ? {
                label: 'Edit layout',
                onClick: onStartEdit,
                leftSection: <IconPencil {...iconSm} />,
              }
            : undefined
        }
      />
    )
  }

  return (
    <Stack gap="lg">
      {view.sections.map((section) => (
        <Stack key={section.id} gap="sm">
          <Title order={4}>{section.title}</Title>
          {section.items.length === 0 ? (
            <Text size="sm" c="dimmed">
              No bookmarks in this section.
            </Text>
          ) : (
            <SimpleGrid cols={{ base: 2, sm: 4, lg: 6, xl: 8 }} spacing="sm">
              {section.items.map((item) => (
                <BookmarkCard
                  key={item.id}
                  bookmark={{ ...item.bookmark, can_write: false }}
                  viewerUserId={viewerUserId}
                  onEdit={() => undefined}
                  onDelete={() => undefined}
                />
              ))}
            </SimpleGrid>
          )}
        </Stack>
      ))}
      <Text size="sm" c="dimmed">
        Browse the full collection in the{' '}
        <Text span component={Link} to="/library" c="violet" inherit>
          library
        </Text>
        .
      </Text>
    </Stack>
  )
}
