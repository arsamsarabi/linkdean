import { type ReactElement } from 'react'
import { Button, Group, Text } from '@mantine/core'
import { IconFolders, IconTags, IconTrash } from '@tabler/icons-react'
import { useLibraryPageModel } from './LibraryPageContext'
import styles from './LibraryPage.module.css'

export function LibraryBulkBar(): ReactElement | null {
  const lib = useLibraryPageModel()
  if (!lib.selectMode) return null

  const { selectedCount, allVisibleSelected, selectableIds } = lib

  return (
    <Group justify="space-between" wrap="wrap" gap="sm" p="sm" className={styles.filterPanel}>
      <Group gap="sm">
        <Text size="sm" fw={600}>
          {selectedCount} selected
        </Text>
        <Button
          size="compact-sm"
          variant="subtle"
          onClick={() =>
            lib.setSelectedIds(allVisibleSelected ? new Set() : new Set(selectableIds))
          }
          disabled={selectableIds.length === 0}
        >
          {allVisibleSelected ? 'Clear all' : 'Select all'}
        </Button>
      </Group>
      <Group gap="xs">
        <Button
          size="compact-sm"
          variant="light"
          leftSection={<IconFolders size={14} stroke={1.5} />}
          disabled={selectedCount === 0}
          onClick={() => lib.setBulkMode('categories')}
        >
          Categories
        </Button>
        <Button
          size="compact-sm"
          variant="light"
          leftSection={<IconTags size={14} stroke={1.5} />}
          disabled={selectedCount === 0}
          onClick={() => lib.setBulkMode('tag')}
        >
          Tag
        </Button>
        <Button
          size="compact-sm"
          color="red"
          variant="light"
          leftSection={<IconTrash size={14} stroke={1.5} />}
          disabled={selectedCount === 0}
          onClick={() => lib.setBulkDeleteOpen(true)}
        >
          Delete
        </Button>
      </Group>
    </Group>
  )
}
