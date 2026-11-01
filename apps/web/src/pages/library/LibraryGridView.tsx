import type { ReactElement } from 'react'
import { SimpleGrid, Stack } from '@mantine/core'
import { IconBookmark, IconBookmarkPlus, IconSearch } from '@tabler/icons-react'
import {
  BookmarkCard,
  BookmarkGridSkeleton,
  EmptyState,
  type BookmarkCardItem,
} from '@/components'
import type { Category } from '@/schemas'
import type { LibraryListItem, SinceFilter } from '@/services'
import { LibraryFilterPanel } from './LibraryFilterPanel'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  loading: boolean
  list: LibraryListItem[]
  searching: boolean
  searchQuery?: string
  emptyLibrary: boolean
  emptyFiltered: boolean
  filterOpen: boolean
  /** Category / tag / since / vault — drives pills + Filter badge. */
  panelFilterActive: boolean
  /** Includes flag toggles — empty-state messaging. */
  anyFilterActive: boolean
  filterPanelId: string
  draftFilterCats: string[]
  draftFilterTags: string[]
  draftFilterSince: SinceFilter | null
  draftVaultOnly: boolean
  filterCats: string[]
  filterTags: string[]
  filterSince: SinceFilter | null
  vaultOnly: boolean
  catLabel: Map<string, string>
  categories: Category[]
  allTags: { value: string; label: string }[]
  onDraftCats: (v: string[]) => void
  onDraftTags: (v: string[]) => void
  onDraftSince: (v: SinceFilter | null) => void
  onDraftVaultOnly: (v: boolean) => void
  onApplyFilter: () => void
  onCloseFilter: () => void
  onClearFilters: () => void
  onRemoveCat: (id: string) => void
  onRemoveTag: (t: string) => void
  onClearSince: () => void
  onClearVault: () => void
  onClearSearchAndFilters: () => void
  onCreate: () => void
  onEdit: (id: string) => void
  onDelete: (id: string) => void
  onFlags?: (
    id: string,
    patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>
  ) => void
  canCreate?: boolean
  viewerUserId?: string
  selectMode?: boolean
  selectedIds?: ReadonlySet<string>
  onToggleSelect?: (id: string) => void
}

export function LibraryGridView({
  loading,
  list,
  searching,
  searchQuery,
  emptyLibrary,
  emptyFiltered,
  filterOpen,
  panelFilterActive,
  anyFilterActive,
  filterPanelId,
  draftFilterCats,
  draftFilterTags,
  draftFilterSince,
  draftVaultOnly,
  filterCats,
  filterTags,
  filterSince,
  vaultOnly,
  catLabel,
  categories,
  allTags,
  onDraftCats,
  onDraftTags,
  onDraftSince,
  onDraftVaultOnly,
  onApplyFilter,
  onCloseFilter,
  onClearFilters,
  onRemoveCat,
  onRemoveTag,
  onClearSince,
  onClearVault,
  onClearSearchAndFilters,
  onCreate,
  onEdit,
  onDelete,
  onFlags,
  canCreate = true,
  viewerUserId,
  selectMode = false,
  selectedIds,
  onToggleSelect,
}: Props): ReactElement {
  return (
    <Stack gap="md">
      <LibraryFilterPanel
        filterOpen={filterOpen}
        panelFilterActive={panelFilterActive}
        filterPanelId={filterPanelId}
        draftFilterCats={draftFilterCats}
        draftFilterTags={draftFilterTags}
        draftFilterSince={draftFilterSince}
        draftVaultOnly={draftVaultOnly}
        filterCats={filterCats}
        filterTags={filterTags}
        filterSince={filterSince}
        vaultOnly={vaultOnly}
        catLabel={catLabel}
        categories={categories}
        allTags={allTags}
        onDraftCats={onDraftCats}
        onDraftTags={onDraftTags}
        onDraftSince={onDraftSince}
        onDraftVaultOnly={onDraftVaultOnly}
        onApplyFilter={onApplyFilter}
        onCloseFilter={onCloseFilter}
        onClearFilters={onClearFilters}
        onRemoveCat={onRemoveCat}
        onRemoveTag={onRemoveTag}
        onClearSince={onClearSince}
        onClearVault={onClearVault}
      />

      {loading ? (
        <BookmarkGridSkeleton />
      ) : emptyLibrary ? (
        <EmptyState
          icon={<IconBookmark size={28} stroke={1.5} />}
          title={canCreate ? 'No bookmarks yet' : 'Nothing here yet'}
          description={
            canCreate
              ? 'Save the first link. Imports live under Config options.'
              : 'This shared folder has no bookmarks yet.'
          }
          action={
            canCreate
              ? {
                  label: 'Add bookmark',
                  onClick: onCreate,
                  leftSection: <IconBookmarkPlus {...iconSm} />,
                }
              : undefined
          }
        />
      ) : emptyFiltered ? (
        <EmptyState
          icon={<IconSearch size={28} stroke={1.5} />}
          title="Nothing matched"
          description="Try a different search or clear filters."
          action={
            anyFilterActive || searchQuery
              ? {
                  label: 'Clear search & filters',
                  onClick: onClearSearchAndFilters,
                }
              : undefined
          }
        />
      ) : (
        <SimpleGrid cols={{ base: 1, sm: 2, lg: 3, xl: 4 }} spacing="lg">
          {list.map((b) => (
            <BookmarkCard
              key={b.id}
              bookmark={b as BookmarkCardItem}
              highlightQuery={searching ? searchQuery : undefined}
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
      )}
    </Stack>
  )
}
