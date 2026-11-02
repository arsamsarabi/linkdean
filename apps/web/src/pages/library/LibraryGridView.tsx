import type { ReactElement } from 'react'
import { Button, Group, SimpleGrid, Stack, Text } from '@mantine/core'
import { IconBookmark, IconBookmarkPlus, IconFolder, IconSearch, IconX } from '@tabler/icons-react'
import { BookmarkCard, BookmarkGridSkeleton, EmptyState, type BookmarkCardItem } from '@/components'
import { FOLDER_ALL } from '@/services'
import { LibraryFilterPanel } from './LibraryFilterPanel'
import { useLibraryPageModel } from './useLibraryPageModel'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LibraryGridView(): ReactElement {
  const lib = useLibraryPageModel()

  return (
    <Stack gap="md">
      <LibraryFilterPanel />

      {lib.folderCat ? (
        <Group gap="xs">
          <IconFolder {...iconSm} />
          <Text size="sm">
            In <strong>{lib.folderCat.name}</strong>
          </Text>
          <Button
            size="compact-xs"
            variant="subtle"
            leftSection={<IconX size={12} stroke={1.5} />}
            onClick={() => lib.setLibrarySearch({ folder: FOLDER_ALL })}
          >
            Show all
          </Button>
        </Group>
      ) : null}

      {lib.loading ? (
        <BookmarkGridSkeleton />
      ) : lib.emptyLibrary ? (
        <EmptyState
          icon={<IconBookmark size={28} stroke={1.5} />}
          title={lib.canCreateInContext ? 'No bookmarks yet' : 'Nothing here yet'}
          description={
            lib.scope === 'vault'
              ? 'Add a bookmark here, or use Move to vault from the Library.'
              : lib.canCreateInContext
                ? 'Save the first link. Imports live under Config options.'
                : 'This shared folder has no bookmarks yet.'
          }
          action={
            lib.canCreateInContext
              ? {
                  label: 'Add bookmark',
                  onClick: lib.openCreate,
                  leftSection: <IconBookmarkPlus {...iconSm} />,
                }
              : undefined
          }
        />
      ) : lib.emptyFiltered ? (
        <EmptyState
          icon={<IconSearch size={28} stroke={1.5} />}
          title="Nothing matched"
          description="Try a different search or clear filters."
          action={
            lib.filterActive || lib.debouncedQ
              ? {
                  label: 'Clear search & filters',
                  onClick: () => {
                    lib.setQ('')
                    lib.setLibrarySearch({
                      q: '',
                      category: [],
                      tag: [],
                      flag: [],
                      since: null,
                      vault: false,
                      folder: FOLDER_ALL,
                    })
                  },
                }
              : undefined
          }
        />
      ) : (
        <SimpleGrid cols={{ base: 1, sm: 2, lg: 3, xl: 4 }} spacing="lg">
          {lib.list.map((b) => (
            <BookmarkCard
              key={b.id}
              bookmark={b as BookmarkCardItem}
              highlightQuery={lib.searching ? lib.debouncedQ : undefined}
              viewerUserId={lib.me.data?.user.id}
              onEdit={lib.openEdit}
              onDelete={lib.setDeleteId}
              onMoveToVault={(id) => lib.setMoveTarget({ categoryIds: [], bookmarkIds: [id] })}
              onFlags={lib.onFlags}
              selectMode={lib.selectMode}
              selected={lib.selectedIds.has(b.id)}
              onToggleSelect={lib.toggleSelect}
            />
          ))}
        </SimpleGrid>
      )}
    </Stack>
  )
}
