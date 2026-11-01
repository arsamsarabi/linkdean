import { type ReactElement } from 'react'
import { Button, Group, Text } from '@mantine/core'
import { IconFolders, IconTags, IconTrash } from '@tabler/icons-react'
import styles from './LibraryPage.module.css'

type Props = {
  selectedCount: number
  allVisibleSelected: boolean
  selectableCount: number
  onToggleSelectAll: () => void
  onCategories: () => void
  onTag: () => void
  onDelete: () => void
}

export function LibraryBulkBar({
  selectedCount,
  allVisibleSelected,
  selectableCount,
  onToggleSelectAll,
  onCategories,
  onTag,
  onDelete,
}: Props): ReactElement {
  return (
    <Group justify="space-between" wrap="wrap" gap="sm" p="sm" className={styles.filterPanel}>
      <Group gap="sm">
        <Text size="sm" fw={600}>
          {selectedCount} selected
        </Text>
        <Button
          size="compact-sm"
          variant="subtle"
          onClick={onToggleSelectAll}
          disabled={selectableCount === 0}
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
          onClick={onCategories}
        >
          Categories
        </Button>
        <Button
          size="compact-sm"
          variant="light"
          leftSection={<IconTags size={14} stroke={1.5} />}
          disabled={selectedCount === 0}
          onClick={onTag}
        >
          Tag
        </Button>
        <Button
          size="compact-sm"
          color="red"
          variant="light"
          leftSection={<IconTrash size={14} stroke={1.5} />}
          disabled={selectedCount === 0}
          onClick={onDelete}
        >
          Delete
        </Button>
      </Group>
    </Group>
  )
}
