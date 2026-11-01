import type { ReactElement } from 'react'
import {
  Button,
  Checkbox,
  Collapse,
  Group,
  MultiSelect,
  Pill,
  Select,
  SimpleGrid,
  Stack,
} from '@mantine/core'
import { IconBookmark, IconBookmarkPlus, IconSearch } from '@tabler/icons-react'
import {
  BookmarkCard,
  BookmarkGridSkeleton,
  EmptyState,
  type BookmarkCardItem,
} from '@/components'
import { categorySelectData } from '@/lib'
import type { Category } from '@/schemas'
import {
  FOLDER_UNCATEGORIZED,
  SINCE_FILTERS,
  type LibraryListItem,
  type SinceFilter,
} from '@/services'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

const SINCE_LABEL: Record<SinceFilter, string> = {
  '7d': 'Last 7 days',
  '30d': 'Last 30 days',
  '90d': 'Last 90 days',
  year: 'Last year',
}

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
  const draftHasFilters =
    draftFilterCats.length > 0 ||
    draftFilterTags.length > 0 ||
    !!draftFilterSince ||
    draftVaultOnly

  return (
    <Stack gap="md">
      <Collapse expanded={filterOpen}>
        <Stack id={filterPanelId} p="lg" gap="md" className={styles.filterPanel}>
          <MultiSelect
            label="Categories"
            data={[{ value: FOLDER_UNCATEGORIZED, label: 'Unsorted' }, ...categorySelectData(categories)]}
            value={draftFilterCats}
            onChange={onDraftCats}
            searchable
            clearable
          />
          <MultiSelect
            label="Tags"
            data={allTags}
            value={draftFilterTags}
            onChange={onDraftTags}
            searchable
            clearable
          />
          <Select
            label="Created"
            placeholder="Any time"
            data={SINCE_FILTERS.map((s) => ({ value: s, label: SINCE_LABEL[s] }))}
            value={draftFilterSince}
            onChange={(v) => onDraftSince(v && (SINCE_FILTERS as readonly string[]).includes(v) ? (v as SinceFilter) : null)}
            clearable
          />
          <Checkbox
            label="Vault only"
            checked={draftVaultOnly}
            onChange={(e) => onDraftVaultOnly(e.currentTarget.checked)}
          />
          <Group justify="space-between" wrap="wrap">
            <Button
              variant="subtle"
              disabled={!draftHasFilters && !panelFilterActive}
              onClick={onClearFilters}
            >
              Clear filters
            </Button>
            <Group gap="sm">
              <Button variant="default" onClick={onCloseFilter}>
                Cancel
              </Button>
              <Button onClick={onApplyFilter}>Apply</Button>
            </Group>
          </Group>
        </Stack>
      </Collapse>

      {panelFilterActive ? (
        <Group gap="xs">
          {filterCats.map((id) => (
            <Pill key={id} withRemoveButton onRemove={() => onRemoveCat(id)}>
              {catLabel.get(id) || id}
            </Pill>
          ))}
          {filterTags.map((t) => (
            <Pill key={t} withRemoveButton onRemove={() => onRemoveTag(t)}>
              #{t}
            </Pill>
          ))}
          {filterSince ? (
            <Pill withRemoveButton onRemove={onClearSince}>
              {SINCE_LABEL[filterSince]}
            </Pill>
          ) : null}
          {vaultOnly ? (
            <Pill withRemoveButton onRemove={onClearVault}>
              Vault only
            </Pill>
          ) : null}
        </Group>
      ) : null}

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
