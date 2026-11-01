import { type ReactElement } from 'react'
import { Stack, Text } from '@mantine/core'
import { IconFolder, IconSearch } from '@tabler/icons-react'
import type { CategoryFolderMenuProps } from '@/components/category/CategoryFolderMenu'
import { EmptyState } from '@/components/empty-state'
import { useLibraryExplorerContents } from '@/hooks/library'
import type { SortKey } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import type { ExplorerFolderId, FlagFilter, LibraryListItem } from '@/services'
import { ExplorerBookmarkGrid } from './ExplorerBookmarkGrid'
import { ExplorerBreadcrumb } from './ExplorerBreadcrumb'
import { ExplorerFolderGrid } from './ExplorerFolderGrid'

type CategoryMenuHandlers = Omit<CategoryFolderMenuProps, 'name' | 'access' | 'isVault' | 'className'>

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
  onFlags?: (id: string, patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>) => void
  filterFlags?: FlagFilter[]
  onCreate: () => void
  canCreate?: boolean
  folderReadOnly?: boolean
  viewerUserId?: string
  selectMode?: boolean
  selectedIds?: ReadonlySet<string>
  onToggleSelect?: (id: string) => void
  /** Category ⋮ menu — same actions as Settings → Categories. */
  categoryMenuFor?: (cat: Category) => CategoryMenuHandlers | undefined
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
  onFlags,
  filterFlags = [],
  onCreate,
  canCreate = true,
  folderReadOnly = false,
  viewerUserId,
  selectMode = false,
  selectedIds,
  onToggleSelect,
  categoryMenuFor,
}: Props): ReactElement {
  const { atAll, byId, crumbs, sortedFolders, ownedFolders, sharedFolders, sortedBookmarks, sortedHits } =
    useLibraryExplorerContents({
      categories,
      bookmarks,
      searchHits,
      folderId,
      sort,
      showHidden,
      filterFlags,
    })

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
        <ExplorerBookmarkGrid
          items={sortedHits}
          highlightQuery={searchQuery}
          viewerUserId={viewerUserId}
          onEdit={onEdit}
          onDelete={onDelete}
          onFlags={onFlags}
          selectMode={selectMode}
          selectedIds={selectedIds}
          onToggleSelect={onToggleSelect}
        />
      </Stack>
    )
  }

  const emptyFolder = sortedFolders.length === 0 && sortedBookmarks.length === 0
  const emptyLibrary = bookmarks.length === 0 && categories.length === 0
  const splitShared = atAll && sharedFolders.length > 0

  return (
    <Stack gap="md">
      <ExplorerBreadcrumb segments={crumbs} onNavigate={onFolderChange} />

      {emptyLibrary ? (
        <EmptyState
          icon={<IconFolder size={28} stroke={1.5} />}
          title="No bookmarks yet"
          description="Save the first link. Imports live under Config options."
          action={canCreate ? { label: 'Add bookmark', onClick: onCreate } : undefined}
        />
      ) : emptyFolder ? (
        <EmptyState
          icon={<IconFolder size={28} stroke={1.5} />}
          title={folderReadOnly ? 'Shared folder is empty' : 'Empty folder'}
          description={
            folderReadOnly ? 'The owner has not added bookmarks here yet.' : 'No subfolders or bookmarks here.'
          }
          action={canCreate ? { label: 'Add bookmark', onClick: onCreate } : undefined}
        />
      ) : (
        <Stack gap="lg">
          {ownedFolders.length > 0 ? (
            <Stack gap="sm">
              <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                {splitShared ? 'Your folders' : 'Folders'}
              </Text>
              <ExplorerFolderGrid
                folders={ownedFolders}
                categoriesById={byId}
                onOpen={onFolderChange}
                categoryMenuFor={categoryMenuFor}
              />
            </Stack>
          ) : null}
          {sharedFolders.length > 0 ? (
            <Stack gap="sm">
              <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                Shared with you
              </Text>
              <ExplorerFolderGrid
                folders={sharedFolders}
                categoriesById={byId}
                onOpen={onFolderChange}
                categoryMenuFor={categoryMenuFor}
              />
            </Stack>
          ) : null}
          {sortedBookmarks.length > 0 ? (
            <Stack gap="sm">
              {sortedFolders.length > 0 ? (
                <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                  Bookmarks
                </Text>
              ) : null}
              <ExplorerBookmarkGrid
                items={sortedBookmarks}
                viewerUserId={viewerUserId}
                onEdit={onEdit}
                onDelete={onDelete}
                onFlags={onFlags}
                selectMode={selectMode}
                selectedIds={selectedIds}
                onToggleSelect={onToggleSelect}
              />
            </Stack>
          ) : null}
        </Stack>
      )}
    </Stack>
  )
}
