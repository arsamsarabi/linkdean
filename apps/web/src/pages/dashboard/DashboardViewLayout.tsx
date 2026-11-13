import { useRef, useState, type ReactElement } from 'react'
import { Group, Kbd, Stack, Text } from '@mantine/core'
import { IconLayoutDashboard, IconPinned } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { EmptyState } from '@/components'
import type { Dashboard } from '@/schemas'
import { filterSections } from '@/services'
import grid from './DashboardGrid.module.css'
import { DashboardSectionPanel } from './DashboardSectionPanel'
import { QuickPinButton } from './QuickPinButton'
import { useCollapsedSections } from './useCollapsedSections'
import { useDashboardKeys } from './useDashboardKeys'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  view: Dashboard
  empty: boolean
  canEditLayout: boolean
  viewerUserId: string | undefined
  folderName: (id: string) => string | undefined
  /** Text from the header's filter box. */
  filter: string
  bookmarkOptions: { value: string; label: string }[]
  quickPinPending: boolean
  onQuickPin: (sectionId: string, bookmarkId: string) => Promise<boolean>
  onStartEdit: () => void
}

export function DashboardViewLayout({
  view,
  empty,
  canEditLayout,
  viewerUserId,
  folderName,
  filter,
  bookmarkOptions,
  quickPinPending,
  onQuickPin,
  onStartEdit,
}: Props): ReactElement {
  const root = useRef<HTMLDivElement>(null)
  const collapsed = useCollapsedSections()
  const [activeId, setActiveId] = useState<string | null>(null)
  useDashboardKeys(root, !empty)

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

  const filtering = filter.trim().length > 0
  const shown = filterSections(view.sections, filter)
  const active = activeId ?? shown.find((s) => s.items.length > 0)?.id ?? shown[0]?.id

  return (
    <Stack gap="xl" ref={root}>
      <div className={grid.grid}>
        {shown.map((section, i) => (
          <div key={section.id} className={grid.cell} data-width={section.width}>
            <DashboardSectionPanel
              section={section}
              totalItems={view.sections[i]?.items.length ?? section.items.length}
              filtering={filtering}
              // A filter shows every match, collapsed or not.
              collapsed={!filtering && collapsed.isCollapsed(section.id)}
              active={section.id === active}
              canEditLayout={canEditLayout}
              viewerUserId={viewerUserId}
              folderName={folderName}
              quickPin={
                canEditLayout && section.kind === 'pinned' ? (
                  <QuickPinButton
                    sectionTitle={section.title}
                    options={bookmarkOptions}
                    pinnedIds={(view.sections[i]?.items ?? []).map((it) => it.bookmark.id)}
                    saving={quickPinPending}
                    onPin={(bookmarkId) => onQuickPin(section.id, bookmarkId)}
                  />
                ) : null
              }
              onToggleCollapse={() => collapsed.toggle(section.id)}
              onActivate={() => setActiveId(section.id)}
              onStartEdit={onStartEdit}
            />
          </div>
        ))}
      </div>
      <Group justify="space-between" gap="sm" c="dimmed">
        <Text size="sm" c="dimmed">
          Browse the full collection in the{' '}
          <Text span component={Link} to="/library" c="violet" inherit>
            library
          </Text>
          .
        </Text>
        <Text size="xs" c="dimmed" visibleFrom="sm">
          Point at a section, then press the number shown on a bookmark to open it. <Kbd size="xs">Tab</Kbd> to a
          bookmark, then <Kbd size="xs">←</Kbd> <Kbd size="xs">→</Kbd> move within its section and{' '}
          <Kbd size="xs">↑</Kbd> <Kbd size="xs">↓</Kbd> jump between sections.
        </Text>
      </Group>
    </Stack>
  )
}
