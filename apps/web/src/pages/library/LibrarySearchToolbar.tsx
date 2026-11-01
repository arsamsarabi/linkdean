import { type ReactElement, type RefObject } from 'react'
import {
  ActionIcon,
  Button,
  Group,
  Kbd,
  Select,
  Stack,
  TextInput,
  Tooltip,
} from '@mantine/core'
import {
  IconBookmark,
  IconFilter,
  IconPin,
  IconSearch,
  IconStar,
  IconX,
} from '@tabler/icons-react'
import type { LibraryViewMode } from '@/hooks'
import { type FlagKey, type SortKey } from '@/lib'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  searchRef: RefObject<HTMLInputElement | null>
  filterPanelId: string
  viewMode: LibraryViewMode
  q: string
  sort: SortKey
  filterFlags: string[]
  filterOpen: boolean
  panelFilterActive: boolean
  panelFilterCount: number
  onQChange: (value: string) => void
  onClearSearch: () => void
  onToggleFilter: () => void
  onToggleFlag: (flag: FlagKey) => void
  onSortChange: (sort: SortKey) => void
}

export function LibrarySearchToolbar({
  searchRef,
  filterPanelId,
  viewMode,
  q,
  sort,
  filterFlags,
  filterOpen,
  panelFilterActive,
  panelFilterCount,
  onQChange,
  onClearSearch,
  onToggleFilter,
  onToggleFlag,
  onSortChange,
}: Props): ReactElement {
  return (
    <Stack gap="md">
      <Group justify="space-between" align="stretch" wrap="wrap" gap="sm">
        <TextInput
          ref={searchRef}
          placeholder="Search bookmarks…"
          leftSection={<IconSearch {...iconSm} />}
          rightSection={
            q ? (
              <ActionIcon
                variant="subtle"
                color="gray"
                aria-label="Clear search"
                onClick={onClearSearch}
              >
                <IconX size={14} stroke={1.5} />
              </ActionIcon>
            ) : (
              <Kbd visibleFrom="sm" size="xs">
                ⌘K
              </Kbd>
            )
          }
          rightSectionPointerEvents={q ? 'all' : 'none'}
          value={q}
          onChange={(e) => onQChange(e.currentTarget.value)}
          className={styles.searchInput}
        />
        <Group gap="xs" wrap="nowrap" align="stretch">
          {viewMode === 'grid' ? (
            <Tooltip label="Filter by category or tag">
              <Button
                variant={filterOpen || panelFilterActive ? 'light' : 'default'}
                leftSection={<IconFilter {...iconSm} />}
                onClick={onToggleFilter}
                aria-expanded={filterOpen}
                aria-controls={filterPanelId}
                h="100%"
              >
                Filter
                {panelFilterActive ? ` (${panelFilterCount})` : ''}
              </Button>
            </Tooltip>
          ) : null}
          <Button.Group className={styles.flagFilters}>
            <Button
              variant={filterFlags.includes('unread') ? 'light' : 'default'}
              leftSection={<IconBookmark {...iconSm} />}
              onClick={() => onToggleFlag('unread')}
              h="100%"
            >
              Reading list
            </Button>
            <Button
              variant={filterFlags.includes('favorite') ? 'light' : 'default'}
              leftSection={<IconStar {...iconSm} />}
              onClick={() => onToggleFlag('favorite')}
              h="100%"
            >
              Favorites
            </Button>
            <Button
              variant={filterFlags.includes('pinned') ? 'light' : 'default'}
              leftSection={<IconPin {...iconSm} />}
              onClick={() => onToggleFlag('pinned')}
              h="100%"
            >
              Pinned
            </Button>
          </Button.Group>
          <Select
            aria-label="Sort"
            value={sort}
            onChange={(v) => v && onSortChange(v as SortKey)}
            allowDeselect={false}
            w={200}
            data={[
              { value: 'updated_desc', label: 'Recently updated' },
              { value: 'updated_asc', label: 'Oldest updated' },
              { value: 'created_desc', label: 'Newest created' },
              { value: 'title_asc', label: 'Title A–Z' },
              { value: 'title_desc', label: 'Title Z–A' },
            ]}
          />
        </Group>
      </Group>
    </Stack>
  )
}
