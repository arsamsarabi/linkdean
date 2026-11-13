import { type CSSProperties, type ReactElement, type ReactNode } from 'react'
import { ActionIcon, Badge, Button, Collapse, Group, Menu, Stack, Text, Tooltip } from '@mantine/core'
import {
  IconChevronDown,
  IconDotsVertical,
  IconExternalLink,
  IconLibrary,
  IconPencil,
  IconSearchOff,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import type { DashboardSection } from '@/schemas'
import { describeSectionSource, sectionLibrarySearch } from '@/services'
import { DashboardSectionItems } from './DashboardSectionItems'
import styles from './DashboardSectionPanel.module.css'
import { SECTION_KIND_LOOK } from './sectionKinds'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  section: DashboardSection
  /** Bookmarks in the section before the dashboard filter. */
  totalItems: number
  filtering: boolean
  collapsed: boolean
  active: boolean
  canEditLayout: boolean
  viewerUserId: string | undefined
  folderName: (id: string) => string | undefined
  /** The "+" that pins a bookmark here without entering edit mode; pinned sections only. */
  quickPin?: ReactNode
  onToggleCollapse: () => void
  onActivate: () => void
  onStartEdit: () => void
}

function EmptyBody({
  section,
  filtering,
  canEditLayout,
  onStartEdit,
}: Pick<Props, 'section' | 'filtering' | 'canEditLayout' | 'onStartEdit'>): ReactElement {
  const look = SECTION_KIND_LOOK[section.kind]
  if (filtering) {
    return (
      <div className={styles.empty}>
        <div className={styles.emptyIcon}>
          <IconSearchOff size={20} stroke={1.6} />
        </div>
        <Text size="sm" c="dimmed">
          Nothing here matches the filter.
        </Text>
      </div>
    )
  }
  const copy: Record<DashboardSection['kind'], { line: string; action: ReactNode }> = {
    pinned: {
      line: 'Pin the links you reach for every day.',
      action: canEditLayout ? (
        <Button size="xs" variant="light" color={look.color} onClick={onStartEdit}>
          Pick bookmarks
        </Button>
      ) : null,
    },
    latest: {
      line: 'No bookmarks match this section yet.',
      action: canEditLayout ? (
        <Button size="xs" variant="light" color={look.color} onClick={onStartEdit}>
          Change its filters
        </Button>
      ) : null,
    },
    most_opened: {
      line: 'Open a few bookmarks and your favorites rise to the top.',
      action: (
        <Button size="xs" variant="light" color={look.color} component={Link} to="/library">
          Open your library
        </Button>
      ),
    },
    recently_visited: {
      line: 'Bookmarks you open show up here, newest first.',
      action: (
        <Button size="xs" variant="light" color={look.color} component={Link} to="/library">
          Open your library
        </Button>
      ),
    },
  }
  const { line, action } = copy[section.kind]
  return (
    <div className={styles.empty}>
      <div className={styles.emptyIcon}>{look.icon(22)}</div>
      <Text size="sm" c="dimmed" maw={360}>
        {line}
      </Text>
      {action}
    </div>
  )
}

/** One dashboard section as a panel: its kind's colour and icon, a header, and its bookmarks. */
export function DashboardSectionPanel({
  section,
  totalItems,
  filtering,
  collapsed,
  active,
  canEditLayout,
  viewerUserId,
  folderName,
  quickPin,
  onToggleCollapse,
  onActivate,
  onStartEdit,
}: Props): ReactElement {
  const look = SECTION_KIND_LOOK[section.kind]
  const caption = describeSectionSource(section, folderName)
  const librarySearch = sectionLibrarySearch(section)
  const bodyId = `dash-section-${section.id}`
  const accent = {
    '--dash-accent-light': `var(--mantine-color-${look.color}-6)`,
    '--dash-accent-dark': `var(--mantine-color-${look.color}-4)`,
  } as CSSProperties

  return (
    <section
      className={styles.panel}
      style={accent}
      aria-labelledby={`${bodyId}-title`}
      data-dash-section
      data-active={active || undefined}
      onMouseEnter={onActivate}
      onFocus={onActivate}
    >
      <Group className={styles.header} justify="space-between" wrap="nowrap" gap="sm" align="center">
        <Group gap="sm" wrap="nowrap" style={{ minWidth: 0 }}>
          <div className={styles.chip} aria-hidden>
            {look.icon(20)}
          </div>
          <Stack gap={2} style={{ minWidth: 0 }}>
            <Group gap={8} wrap="nowrap">
              <Text id={`${bodyId}-title`} className={styles.title} component="h2" lineClamp={1}>
                {section.title}
              </Text>
              <Badge size="sm" variant="light" color={look.color} radius="sm">
                {filtering && section.items.length !== totalItems
                  ? `${section.items.length} of ${totalItems}`
                  : totalItems}
              </Badge>
              {section.items.length > 0 ? (
                <span className={styles.keysChip}>Keys 1–{Math.min(9, section.items.length)} open these</span>
              ) : null}
            </Group>
            <Text size="xs" c="dimmed" lineClamp={1}>
              {caption ?? `${look.label} · bookmarks you picked`}
            </Text>
          </Stack>
        </Group>
        <Group gap={4} wrap="nowrap">
          {quickPin}
          <Menu position="bottom-end" width={220} withinPortal>
            <Menu.Target>
              <ActionIcon variant="subtle" color="gray" aria-label={`${section.title} actions`}>
                <IconDotsVertical {...iconSm} />
              </ActionIcon>
            </Menu.Target>
            <Menu.Dropdown>
              {librarySearch ? (
                <Menu.Item
                  leftSection={<IconLibrary size={14} stroke={1.5} />}
                  rightSection={<IconExternalLink size={12} stroke={1.5} />}
                  renderRoot={(props) => <Link {...props} to="/library" search={librarySearch} />}
                >
                  See all in library
                </Menu.Item>
              ) : null}
              {canEditLayout ? (
                <Menu.Item leftSection={<IconPencil size={14} stroke={1.5} />} onClick={onStartEdit}>
                  Edit layout
                </Menu.Item>
              ) : null}
              <Menu.Item leftSection={<IconChevronDown size={14} stroke={1.5} />} onClick={onToggleCollapse}>
                {collapsed ? 'Expand' : 'Collapse'}
              </Menu.Item>
            </Menu.Dropdown>
          </Menu>
          <Tooltip label={collapsed ? 'Expand' : 'Collapse'}>
            <ActionIcon
              variant="subtle"
              color="gray"
              aria-expanded={!collapsed}
              aria-controls={bodyId}
              aria-label={collapsed ? `Expand ${section.title}` : `Collapse ${section.title}`}
              onClick={onToggleCollapse}
            >
              <IconChevronDown
                {...iconSm}
                style={{ transform: collapsed ? 'rotate(-90deg)' : undefined, transition: 'transform 160ms ease' }}
              />
            </ActionIcon>
          </Tooltip>
        </Group>
      </Group>
      <Collapse expanded={!collapsed} id={bodyId}>
        <div className={styles.body}>
          {section.items.length === 0 ? (
            <EmptyBody
              section={section}
              filtering={filtering}
              canEditLayout={canEditLayout}
              onStartEdit={onStartEdit}
            />
          ) : (
            <DashboardSectionItems
              kind={section.kind}
              display={section.display}
              items={section.items}
              viewerUserId={viewerUserId}
            />
          )}
        </div>
      </Collapse>
    </section>
  )
}
