import { useMemo, type ReactElement } from 'react'
import { SimpleGrid, Stack, Text } from '@mantine/core'
import { IconFolder, IconSearch } from '@tabler/icons-react'
import { BookmarkCard } from '@/components/bookmark'
import type { CategoryFolderMenuProps } from '@/components/category/CategoryFolderMenu'
import { EmptyState } from '@/components/empty-state'
import type { SortKey } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import {
  FOLDER_ALL,
  filterAndSortLibraryItems,
  folderBreadcrumb,
  isSharedFolder,
  listFolderContents,
  sortExplorerFolders,
  type ExplorerFolder,
  type ExplorerFolderId,
  type FlagFilter,
  type LibraryListItem,
} from '@/services'
import { ExplorerBreadcrumb } from './ExplorerBreadcrumb'
import { ExplorerFolderTile } from './ExplorerFolderTile'

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
  onFlags?: (
    id: string,
    patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>
  ) => void
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

function BookmarkGrid({
  items,
  highlightQuery,
  viewerUserId,
  onEdit,
  onDelete,
  onFlags,
  selectMode,
  selectedIds,
  onToggleSelect,
}: {
  items: LibraryListItem[]
  highlightQuery?: string
  viewerUserId?: string
  onEdit: (id: string) => void
  onDelete: (id: string) => void
  onFlags?: (
    id: string,
    patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>
  ) => void
  selectMode?: boolean
  selectedIds?: ReadonlySet<string>
  onToggleSelect?: (id: string) => void
}): ReactElement {
  return (
    <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="lg">
      {items.map((b) => (
        <BookmarkCard
          key={b.id}
          bookmark={b}
          highlightQuery={highlightQuery}
          viewerUserId={viewerUserId}
          onEdit={onEdit}
          onDelete={onDelete}
          onFlags={onFlags}
          selectMode={selectMode}
          selected={selectedIds?.has(b.id)}
          onToggleSelect={onToggleSelect}
        />
      ))}
    </SimpleGrid>
  )
}

function FolderGrid({
  folders,
  categoriesById,
  onOpen,
  categoryMenuFor,
}: {
  folders: ExplorerFolder[]
  categoriesById: Map<string, Category>
  onOpen: (id: ExplorerFolderId) => void
  categoryMenuFor?: (cat: Category) => CategoryMenuHandlers | undefined
}): ReactElement {
  return (
    <SimpleGrid cols={{ base: 2, sm: 3, md: 4, lg: 5 }} spacing="md">
      {folders.map((f) => {
        const cat = categoriesById.get(f.id)
        const menu = cat && categoryMenuFor ? categoryMenuFor(cat) : undefined
        return <ExplorerFolderTile key={f.id} folder={f} onOpen={onOpen} menu={menu} />
      })}
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
  const selectedId: ExplorerFolderId = folderId || FOLDER_ALL
  const atAll = selectedId === FOLDER_ALL

  const byId = useMemo(() => new Map(categories.map((c) => [c.id, c])), [categories])
  const crumbs = useMemo(() => folderBreadcrumb(selectedId, byId), [selectedId, byId])
  const contents = useMemo(
    () => listFolderContents(selectedId, categories, bookmarks, showHidden),
    [selectedId, categories, bookmarks, showHidden]
  )
  const sortedFolders = useMemo(
    () => sortExplorerFolders(contents.folders, sort),
    [contents.folders, sort]
  )
  const ownedFolders = useMemo(
    () => (atAll ? sortedFolders.filter((f) => !isSharedFolder(f)) : sortedFolders),
    [atAll, sortedFolders]
  )
  const sharedFolders = useMemo(
    () => (atAll ? sortedFolders.filter(isSharedFolder) : []),
    [atAll, sortedFolders]
  )
  const sortedBookmarks = useMemo(
    () =>
      filterAndSortLibraryItems({
        items: contents.bookmarks,
        filterCats: [],
        filterTags: [],
        filterFlags,
        sort,
      }),
    [contents.bookmarks, filterFlags, sort]
  )
  const sortedHits = useMemo(
    () =>
      filterAndSortLibraryItems({
        items: searchHits,
        filterCats: [],
        filterTags: [],
        filterFlags,
        sort,
      }),
    [searchHits, filterFlags, sort]
  )

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
        <BookmarkGrid
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
            folderReadOnly
              ? 'The owner has not added bookmarks here yet.'
              : 'No subfolders or bookmarks here.'
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
              <FolderGrid
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
              <FolderGrid
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
              <BookmarkGrid
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
