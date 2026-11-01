import type { ReactElement } from 'react'
import { SimpleGrid, Stack } from '@mantine/core'
import { IconBookmark, IconBookmarkPlus, IconSearch } from '@tabler/icons-react'
import {
  BookmarkCard,
  BookmarkGridSkeleton,
  EmptyState,
  type BookmarkCardItem,
} from '@/components'
import { useLibraryPageModel } from './LibraryPageContext'
import { LibraryFilterPanel } from './LibraryFilterPanel'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LibraryGridView(): ReactElement {
  const lib = useLibraryPageModel()

  return (
    <Stack gap="md">
      <LibraryFilterPanel />

      {lib.loading ? (
        <BookmarkGridSkeleton />
      ) : lib.emptyLibrary ? (
        <EmptyState
          icon={<IconBookmark size={28} stroke={1.5} />}
          title={lib.canCreateInContext ? 'No bookmarks yet' : 'Nothing here yet'}
          description={
            lib.canCreateInContext
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
