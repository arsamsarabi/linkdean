import { useMemo, useState, type ReactElement } from 'react'
import { Button, Drawer, SimpleGrid, Stack, Text } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import { IconFolder, IconSearch } from '@tabler/icons-react'
import { BookmarkCard } from '@/components/bookmark'
import { EmptyState } from '@/components/empty-state'
import type { SortKey } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import {
  FOLDER_ALL,
  buildExplorerTree,
  filterAndSortLibraryItems,
  folderBreadcrumb,
  listFolderContents,
  type ExplorerFolderId,
  type LibraryListItem,
} from '@/services'
import { ExplorerBreadcrumb } from './ExplorerBreadcrumb'
import { ExplorerFolderTile } from './ExplorerFolderTile'
import { ExplorerFolderTree } from './ExplorerFolderTree'
import styles from './LibraryExplorerView.module.css'

type Props = {
  categories: Category[]
  bookmarks: Bookmark[]
  searchHits: LibraryListItem[]
  searching: boolean
  searchQuery?: string
  folderId: ExplorerFolderId | undefined
  sort: SortKey
  showHidden: boolean
  loading: boolean
  onFolderChange: (id: ExplorerFolderId) => void
  onEdit: (id: string) => void
  onDelete: (id: string) => void
  onCreate: () => void
}

function BookmarkGrid({
  items,
  highlightQuery,
  onEdit,
  onDelete,
}: {
  items: LibraryListItem[]
  highlightQuery?: string
  onEdit: (id: string) => void
  onDelete: (id: string) => void
}): ReactElement {
  return (
    <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="lg">
      {items.map((b) => (
        <BookmarkCard key={b.id} bookmark={b} highlightQuery={highlightQuery} onEdit={onEdit} onDelete={onDelete} />
      ))}
    </SimpleGrid>
  )
}

export function LibraryExplorerView({
  categories,
  bookmarks,
  searchHits,
  searching,
  searchQuery,
  folderId,
  sort,
  showHidden,
  loading,
  onFolderChange,
  onEdit,
  onDelete,
  onCreate,
}: Props): ReactElement {
  const isMobile = useMediaQuery('(max-width: 47.99em)')
  const [treeOpen, setTreeOpen] = useState(false)
  const selectedId: ExplorerFolderId = folderId || FOLDER_ALL

  const byId = useMemo(() => new Map(categories.map((c) => [c.id, c])), [categories])
  const tree = useMemo(
    () => buildExplorerTree(categories, bookmarks, showHidden),
    [categories, bookmarks, showHidden]
  )
  const crumbs = useMemo(() => folderBreadcrumb(selectedId, byId), [selectedId, byId])
  const contents = useMemo(
    () => listFolderContents(selectedId, categories, bookmarks, showHidden),
    [selectedId, categories, bookmarks, showHidden]
  )
  const sortedBookmarks = useMemo(
    () => filterAndSortLibraryItems({ items: contents.bookmarks, filterCats: [], filterTags: [], sort }),
    [contents.bookmarks, sort]
  )
  const sortedHits = useMemo(
    () => filterAndSortLibraryItems({ items: searchHits, filterCats: [], filterTags: [], sort }),
    [searchHits, sort]
  )

  function selectFolder(id: ExplorerFolderId): void {
    onFolderChange(id)
    setTreeOpen(false)
  }

  if (loading) {
    return (
      <Text size="sm" c="dimmed">
        Loading folders…
      </Text>
    )
  }

  if (searching) {
    if (sortedHits.length === 0) {
      return (
        <EmptyState
          icon={<IconSearch size={28} stroke={1.5} />}
          title="Nothing matched"
          description="Try a different search."
        />
      )
    }
    return (
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          {sortedHits.length} result{sortedHits.length === 1 ? '' : 's'}
        </Text>
        <BookmarkGrid items={sortedHits} highlightQuery={searchQuery} onEdit={onEdit} onDelete={onDelete} />
      </Stack>
    )
  }

  const emptyFolder = contents.folders.length === 0 && sortedBookmarks.length === 0
  const emptyLibrary = bookmarks.length === 0 && categories.length === 0

  const content = (
    <Stack gap="md" className={styles.contentPane}>
      <ExplorerBreadcrumb segments={crumbs} onNavigate={selectFolder} />

      {emptyLibrary ? (
        <EmptyState
          icon={<IconFolder size={28} stroke={1.5} />}
          title="No bookmarks yet"
          description="Save the first link. Imports live under Settings."
          action={{ label: 'Add bookmark', onClick: onCreate }}
        />
      ) : emptyFolder ? (
        <EmptyState
          icon={<IconFolder size={28} stroke={1.5} />}
          title="Empty folder"
          description="No subfolders or bookmarks here."
          action={{ label: 'Add bookmark', onClick: onCreate }}
        />
      ) : (
        <Stack gap="lg">
          {contents.folders.length > 0 ? (
            <Stack gap="sm">
              <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                Folders
              </Text>
              <SimpleGrid cols={{ base: 2, sm: 3, md: 4 }} spacing="md">
                {contents.folders.map((f) => (
                  <ExplorerFolderTile key={f.id} folder={f} onOpen={selectFolder} />
                ))}
              </SimpleGrid>
            </Stack>
          ) : null}
          {sortedBookmarks.length > 0 ? (
            <Stack gap="sm">
              {contents.folders.length > 0 ? (
                <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                  Bookmarks
                </Text>
              ) : null}
              <BookmarkGrid items={sortedBookmarks} onEdit={onEdit} onDelete={onDelete} />
            </Stack>
          ) : null}
        </Stack>
      )}
    </Stack>
  )

  if (isMobile) {
    return (
      <Stack gap="md">
        <Button
          className={styles.mobileTreeBtn}
          variant="default"
          leftSection={<IconFolder size={16} stroke={1.5} />}
          onClick={() => setTreeOpen(true)}
        >
          Folders
        </Button>
        {content}
        <Drawer opened={treeOpen} onClose={() => setTreeOpen(false)} title="Folders" position="left" size="xs">
          <ExplorerFolderTree tree={tree} selectedId={selectedId} onSelect={selectFolder} />
        </Drawer>
      </Stack>
    )
  }

  return (
    <div className={styles.root}>
      <aside className={`${styles.treePane} ${styles.treePaneDesktop}`} aria-label="Folder tree">
        <ExplorerFolderTree tree={tree} selectedId={selectedId} onSelect={selectFolder} />
      </aside>
      {content}
    </div>
  )
}
