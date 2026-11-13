import { type ReactElement } from 'react'
import { Stack, Text, Title } from '@mantine/core'
import { IconLayoutDashboard, IconPinned } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { EmptyState } from '@/components'
import type { Dashboard, DashboardSection } from '@/schemas'
import { describeSectionSource } from '@/services'
import { DashboardSectionItems } from './DashboardSectionItems'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  view: Dashboard
  empty: boolean
  canEditLayout: boolean
  viewerUserId: string | undefined
  folderName: (id: string) => string | undefined
  onStartEdit: () => void
}

function emptyText(section: DashboardSection): string {
  if (section.kind === 'pinned') return 'Nothing in this section yet.'
  if (section.kind === 'latest') return 'No bookmarks match yet.'
  return 'Open a few bookmarks and they show up here.'
}

export function DashboardViewLayout({
  view,
  empty,
  canEditLayout,
  viewerUserId,
  folderName,
  onStartEdit,
}: Props): ReactElement {
  if (empty) {
    return (
      <EmptyState
        icon={<IconLayoutDashboard size={28} stroke={1.5} />}
        title="Dashboard is empty"
        description={
          canEditLayout
            ? 'Pin links into sections for a quick glance home.'
            : 'This shared dashboard has no sections yet.'
        }
        action={
          canEditLayout
            ? {
                // Names the outcome; the header's Edit layout stays the general entry point.
                label: 'Pin your first links',
                onClick: onStartEdit,
                leftSection: <IconPinned {...iconSm} />,
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
          <Stack gap={2}>
            <Title order={4}>{section.title}</Title>
            {describeSectionSource(section, folderName) ? (
              <Text size="xs" c="dimmed">
                {describeSectionSource(section, folderName)}
              </Text>
            ) : null}
          </Stack>
          {section.items.length === 0 ? (
            <Text size="sm" c="dimmed">
              {emptyText(section)}
            </Text>
          ) : (
            <DashboardSectionItems
              display={section.display}
              bookmarks={section.items.map((i) => i.bookmark)}
              viewerUserId={viewerUserId}
            />
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
