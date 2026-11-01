import { type ReactElement } from 'react'
import { ActionIcon, Button, Group, Kbd, Select, Stack, TextInput, Tooltip } from '@mantine/core'
import { IconBookmark, IconFilter, IconPin, IconSearch, IconStar, IconX } from '@tabler/icons-react'
import { type SortKey } from '@/lib'
import { LIBRARY_FILTER_PANEL_ID } from './libraryConstants'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './LibraryPageContext'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LibrarySearchToolbar(): ReactElement {
  const lib = useLibraryPageModel()

  return (
    <Stack gap="md">
      <Group justify="space-between" align="stretch" wrap="wrap" gap="sm">
        <TextInput
          ref={lib.searchRef}
          placeholder="Search bookmarks…"
          leftSection={<IconSearch {...iconSm} />}
          rightSection={
            lib.q ? (
              <ActionIcon
                variant="subtle"
                color="gray"
                aria-label="Clear search"
                onClick={() => {
                  lib.setQ('')
                  lib.setLibrarySearch({ q: '' })
                }}
              >
                <IconX size={14} stroke={1.5} />
              </ActionIcon>
            ) : (
              <Kbd visibleFrom="sm" size="xs">
                ⌘K
              </Kbd>
            )
          }
          rightSectionPointerEvents={lib.q ? 'all' : 'none'}
          value={lib.q}
          onChange={(e) => lib.setQ(e.currentTarget.value)}
          className={styles.searchInput}
        />
        <Group gap="xs" wrap="nowrap" align="stretch">
          {lib.viewMode === 'grid' ? (
            <Tooltip label="Filter by category or tag">
              <Button
                variant={lib.filterOpen || lib.panelFilterActive ? 'light' : 'default'}
                leftSection={<IconFilter {...iconSm} />}
                onClick={lib.toggleFilter}
                aria-expanded={lib.filterOpen}
                aria-controls={LIBRARY_FILTER_PANEL_ID}
                h="100%"
              >
                Filter
                {lib.panelFilterActive ? ` (${lib.panelFilterCount})` : ''}
              </Button>
            </Tooltip>
          ) : null}
          <Button.Group className={styles.flagFilters}>
            <Button
              variant={lib.filterFlags.includes('unread') ? 'light' : 'default'}
              leftSection={<IconBookmark {...iconSm} />}
              onClick={() => lib.toggleFlagFilter('unread')}
              h="100%"
            >
              Reading list
            </Button>
            <Button
              variant={lib.filterFlags.includes('favorite') ? 'light' : 'default'}
              leftSection={<IconStar {...iconSm} />}
              onClick={() => lib.toggleFlagFilter('favorite')}
              h="100%"
            >
              Favorites
            </Button>
            <Button
              variant={lib.filterFlags.includes('pinned') ? 'light' : 'default'}
              leftSection={<IconPin {...iconSm} />}
              onClick={() => lib.toggleFlagFilter('pinned')}
              h="100%"
            >
              Pinned
            </Button>
          </Button.Group>
          <Select
            aria-label="Sort"
            value={lib.sort}
            onChange={(v) => v && lib.setLibrarySearch({ sort: v as SortKey })}
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
