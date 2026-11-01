import type { ReactElement } from 'react'
import { Button, Checkbox, Collapse, Group, MultiSelect, Pill, Select, Stack } from '@mantine/core'
import { categorySelectData } from '@/lib'
import { FOLDER_UNCATEGORIZED, SINCE_FILTERS, type SinceFilter } from '@/services'
import { LIBRARY_FILTER_PANEL_ID } from './libraryConstants'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './useLibraryPageModel'

const SINCE_LABEL: Record<SinceFilter, string> = {
  '7d': 'Last 7 days',
  '30d': 'Last 30 days',
  '90d': 'Last 90 days',
  year: 'Last year',
}

export function LibraryFilterPanel(): ReactElement {
  const lib = useLibraryPageModel()
  const draftHasFilters =
    lib.draftFilterCats.length > 0 || lib.draftFilterTags.length > 0 || !!lib.draftFilterSince || lib.draftVaultOnly

  return (
    <>
      <Collapse expanded={lib.filterOpen}>
        <Stack id={LIBRARY_FILTER_PANEL_ID} p="lg" gap="md" className={styles.filterPanel}>
          <MultiSelect
            label="Categories"
            data={[{ value: FOLDER_UNCATEGORIZED, label: 'Unsorted' }, ...categorySelectData(lib.cats.data || [])]}
            value={lib.draftFilterCats}
            onChange={lib.setDraftFilterCats}
            searchable
            clearable
          />
          <MultiSelect
            label="Tags"
            data={lib.allTags}
            value={lib.draftFilterTags}
            onChange={lib.setDraftFilterTags}
            searchable
            clearable
          />
          <Select
            label="Created"
            placeholder="Any time"
            data={SINCE_FILTERS.map((s) => ({ value: s, label: SINCE_LABEL[s] }))}
            value={lib.draftFilterSince}
            onChange={(v) =>
              lib.setDraftFilterSince(v && (SINCE_FILTERS as readonly string[]).includes(v) ? (v as SinceFilter) : null)
            }
            clearable
          />
          {lib.vaultOnlyAvailable ? (
            <Checkbox
              label="Vault only"
              checked={lib.draftVaultOnly}
              onChange={(e) => lib.setDraftVaultOnly(e.currentTarget.checked)}
            />
          ) : null}
          <Group justify="space-between" wrap="wrap">
            <Button
              variant="subtle"
              disabled={!draftHasFilters && !lib.panelFilterActive}
              onClick={() => {
                lib.setDraftFilterCats([])
                lib.setDraftFilterTags([])
                lib.setDraftFilterSince(null)
                lib.setDraftVaultOnly(false)
                lib.setLibrarySearch({ category: [], tag: [], since: null, vault: false })
              }}
            >
              Clear filters
            </Button>
            <Group gap="sm">
              <Button variant="default" onClick={() => lib.setFilterOpen(false)}>
                Cancel
              </Button>
              <Button onClick={lib.applyFilter}>Apply</Button>
            </Group>
          </Group>
        </Stack>
      </Collapse>

      {lib.panelFilterActive ? (
        <Group gap="xs">
          {lib.filterCats.map((id) => (
            <Pill
              key={id}
              withRemoveButton
              onRemove={() => lib.setLibrarySearch({ category: lib.filterCats.filter((x) => x !== id) })}
            >
              {lib.catLabel.get(id) || id}
            </Pill>
          ))}
          {lib.filterTags.map((t) => (
            <Pill
              key={t}
              withRemoveButton
              onRemove={() => lib.setLibrarySearch({ tag: lib.filterTags.filter((x) => x !== t) })}
            >
              #{t}
            </Pill>
          ))}
          {lib.filterSince ? (
            <Pill withRemoveButton onRemove={() => lib.setLibrarySearch({ since: null })}>
              {SINCE_LABEL[lib.filterSince]}
            </Pill>
          ) : null}
          {lib.vaultOnlyAvailable && lib.vaultOnly ? (
            <Pill withRemoveButton onRemove={() => lib.setLibrarySearch({ vault: false })}>
              Vault only
            </Pill>
          ) : null}
        </Group>
      ) : null}
    </>
  )
}
