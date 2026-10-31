import type { ReactElement } from 'react'
import {
  Button,
  Collapse,
  Group,
  MultiSelect,
  Pill,
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
import type { LibraryListItem } from '@/services'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  loading: boolean
  list: LibraryListItem[]
  searching: boolean
  searchQuery?: string
  emptyLibrary: boolean
  emptyFiltered: boolean
  filterOpen: boolean
  filterActive: boolean
  filterPanelId: string
  draftFilterCats: string[]
  draftFilterTags: string[]
  filterCats: string[]
  filterTags: string[]
  catLabel: Map<string, string>
  categories: Category[]
  allTags: { value: string; label: string }[]
  onDraftCats: (v: string[]) => void
  onDraftTags: (v: string[]) => void
  onApplyFilter: () => void
  onCloseFilter: () => void
  onClearFilters: () => void
  onRemoveCat: (id: string) => void
  onRemoveTag: (t: string) => void
  onClearSearchAndFilters: () => void
  onCreate: () => void
  onEdit: (id: string) => void
  onDelete: (id: string) => void
}

export function LibraryGridView({
  loading,
  list,
  searching,
  searchQuery,
  emptyLibrary,
  emptyFiltered,
  filterOpen,
  filterActive,
  filterPanelId,
  draftFilterCats,
  draftFilterTags,
  filterCats,
  filterTags,
  catLabel,
  categories,
  allTags,
  onDraftCats,
  onDraftTags,
  onApplyFilter,
  onCloseFilter,
  onClearFilters,
  onRemoveCat,
  onRemoveTag,
  onClearSearchAndFilters,
  onCreate,
  onEdit,
  onDelete,
}: Props): ReactElement {
  return (
    <Stack gap="md">
      <Collapse expanded={filterOpen}>
        <Stack id={filterPanelId} p="lg" gap="md" className={styles.filterPanel}>
          <MultiSelect
            label="Categories"
            data={categorySelectData(categories)}
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
          <Group justify="flex-end">
            <Button variant="default" onClick={onCloseFilter}>
              Cancel
            </Button>
            <Button onClick={onApplyFilter}>Apply</Button>
          </Group>
        </Stack>
      </Collapse>

      {filterActive ? (
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
          <Button variant="subtle" size="compact-sm" onClick={onClearFilters}>
            Clear filters
          </Button>
        </Group>
      ) : null}

      {loading ? (
        <BookmarkGridSkeleton />
      ) : emptyLibrary ? (
        <EmptyState
          icon={<IconBookmark size={28} stroke={1.5} />}
          title="No bookmarks yet"
          description="Save the first link. Imports live under Settings."
          action={{
            label: 'Add bookmark',
            onClick: onCreate,
            leftSection: <IconBookmarkPlus {...iconSm} />,
          }}
        />
      ) : emptyFiltered ? (
        <EmptyState
          icon={<IconSearch size={28} stroke={1.5} />}
          title="Nothing matched"
          description="Try a different search or clear filters."
          action={
            filterActive || searchQuery
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
              onEdit={onEdit}
              onDelete={onDelete}
            />
          ))}
        </SimpleGrid>
      )}
    </Stack>
  )
}
