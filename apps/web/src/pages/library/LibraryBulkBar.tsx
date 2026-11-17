import { type ReactElement } from 'react'
import { Button, Group, Text } from '@mantine/core'
import { IconFolders, IconLock, IconTags, IconTrash } from '@tabler/icons-react'
import { useMe } from '@/hooks'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './useLibraryPageModel'

export function LibraryBulkBar(): ReactElement | null {
  const lib = useLibraryPageModel()
  const me = useMe()
  if (!lib.selectMode) return null

  const { selectedCount, allSelected } = lib
  // Only your own bookmarks can go in a vault; ones shared with you stay where they are.
  // shortcut: rows beyond the loaded pages (Select all) are not screened, since `/bookmarks/ids`
  // carries no owner, and one of those fails the whole move. Return owners there if that bites.
  const notMine = new Set(lib.selectedItems.filter((b) => b.owner_user_id !== me.data?.user.id).map((b) => b.id))
  const vaultIds = [...lib.selectedIds].filter((id) => !notMine.has(id))

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
          disabled={vaultIds.length === 0}
          title={notMine.size > 0 ? 'Bookmarks shared with you cannot go in a vault' : undefined}
          onClick={() => lib.setMoveTarget({ categoryIds: [], bookmarkIds: vaultIds })}
        >
          {notMine.size > 0 && vaultIds.length > 0 ? `Move ${vaultIds.length} to vault` : 'Move to vault'}
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
