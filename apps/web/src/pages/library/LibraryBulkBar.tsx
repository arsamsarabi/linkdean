import { type ReactElement } from 'react'
import { Button, Group, Text } from '@mantine/core'
import { IconFolders, IconLock, IconTags, IconTrash } from '@tabler/icons-react'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './useLibraryPageModel'

// Known issue: Move to vault is offered for shared bookmarks the API refuses, and one refusal fails
// the whole batch. Fix: send owned ids only, or disable the action with a reason.
export function LibraryBulkBar(): ReactElement | null {
  const lib = useLibraryPageModel()
  if (!lib.selectMode) return null

  const { selectedCount, allSelected } = lib

  return (
    <Group justify="space-between" wrap="wrap" gap="sm" p="sm" className={`${styles.filterPanel} ${styles.bulkBar}`}>
      <Group gap="sm">
        <Text size="sm" fw={600}>
          {selectedCount} selected
        </Text>
        <Button
          size="compact-sm"
          variant="subtle"
          onClick={() => void lib.toggleSelectAll()}
          loading={lib.selectingAll}
          disabled={lib.selectableIds.length === 0}
        >
          {allSelected ? 'Clear all' : `Select all${lib.matchFilter ? ` ${lib.total}` : ''}`}
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
          variant="light"
          leftSection={<IconLock size={14} stroke={1.5} />}
          disabled={selectedCount === 0}
          onClick={() => lib.setMoveTarget({ categoryIds: [], bookmarkIds: [...lib.selectedIds] })}
        >
          Move to vault
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
