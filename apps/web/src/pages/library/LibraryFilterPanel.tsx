import type { ReactElement } from 'react'
import {
  Button,
  Checkbox,
  Collapse,
  Group,
  MultiSelect,
  Pill,
  Select,
  Stack,
} from '@mantine/core'
import { categorySelectData } from '@/lib'
import type { Category } from '@/schemas'
import { FOLDER_UNCATEGORIZED, SINCE_FILTERS, type SinceFilter } from '@/services'
import styles from './LibraryPage.module.css'

const SINCE_LABEL: Record<SinceFilter, string> = {
  '7d': 'Last 7 days',
  '30d': 'Last 30 days',
  '90d': 'Last 90 days',
  year: 'Last year',
}

type Props = {
  filterOpen: boolean
  panelFilterActive: boolean
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
}

export function LibraryFilterPanel({
  filterOpen,
  panelFilterActive,
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
}: Props): ReactElement {
  const draftHasFilters =
    draftFilterCats.length > 0 ||
    draftFilterTags.length > 0 ||
    !!draftFilterSince ||
    draftVaultOnly

  return (
    <>
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
            onChange={(v) =>
              onDraftSince(
                v && (SINCE_FILTERS as readonly string[]).includes(v) ? (v as SinceFilter) : null
              )
            }
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
    </>
  )
}
