import { type ReactElement } from 'react'
import {
  ActionIcon,
  Affix,
  Alert,
  Button,
  Group,
  Stack,
  Text,
  Tooltip,
  Transition,
} from '@mantine/core'
import { IconArrowUp, IconBookmarkPlus } from '@tabler/icons-react'
import { LibraryExplorerView, QueryErrorAlert } from '@/components'
import { useLibraryPage } from '@/hooks'
import { canWriteCategory } from '@/lib'
import { LibraryBulkBar } from './LibraryBulkBar'
import { LibraryGridView } from './LibraryGridView'
import { LibraryPageHeader } from './LibraryPageHeader'
import { LibraryPageModals } from './LibraryPageModals'
import { LibrarySearchToolbar } from './LibrarySearchToolbar'
import styles from './LibraryPage.module.css'

const FILTER_PANEL_ID = 'library-filter-panel'

export function LibraryPage(): ReactElement {
  const lib = useLibraryPage()

  return (
    <Stack gap="xl">
      <LibraryPageHeader
        loading={lib.loading}
        viewMode={lib.viewMode}
        listCount={lib.list.length}
        searching={lib.searching}
        selectMode={lib.selectMode}
        canCreate={lib.canCreateInContext}
        onViewModeChange={lib.onViewModeChange}
        onEnterSelect={lib.enterSelectMode}
        onExitSelect={lib.exitSelectMode}
        onOpenDupes={lib.openDupes}
        onCreate={lib.openCreate}
      />

      {lib.selectMode ? (
        <LibraryBulkBar
          selectedCount={lib.selectedCount}
          allVisibleSelected={lib.allVisibleSelected}
          selectableCount={lib.selectableIds.length}
          onToggleSelectAll={() =>
            lib.setSelectedIds(lib.allVisibleSelected ? new Set() : new Set(lib.selectableIds))
          }
          onCategories={() => lib.setBulkMode('categories')}
          onTag={() => lib.setBulkMode('tag')}
          onDelete={() => lib.setBulkDeleteOpen(true)}
        />
      ) : null}

      {lib.cats.isError ? (
        <QueryErrorAlert
          error={lib.cats.error}
          onRetry={() => void lib.cats.refetch()}
          title="Categories failed"
        />
      ) : null}
      {lib.bookmarks.isError && !lib.searching ? (
        <QueryErrorAlert
          error={lib.bookmarks.error}
          onRetry={() => void lib.bookmarks.refetch()}
          title="Bookmarks failed"
        />
      ) : null}
      {lib.search.isError && lib.searching ? (
        <QueryErrorAlert
          error={lib.search.error}
          onRetry={() => void lib.search.refetch()}
          title="Search failed"
        />
      ) : null}
      {lib.searching && lib.searchCatchUpCount > 0 ? (
        <Alert color="yellow" title="Search index catching up" withCloseButton={false}>
          <Group justify="space-between" align="center" wrap="wrap" gap="sm">
            <Text size="sm">
              Showing {lib.searchCatchUpCount} local match
              {lib.searchCatchUpCount === 1 ? '' : 'es'} while Meilisearch finishes indexing.
            </Text>
            <Button
              size="compact-sm"
              variant="light"
              loading={lib.search.isFetching}
              onClick={() => void lib.search.refetch()}
            >
              Retry search
            </Button>
          </Group>
        </Alert>
      ) : null}

      <LibrarySearchToolbar
        searchRef={lib.searchRef}
        filterPanelId={FILTER_PANEL_ID}
        viewMode={lib.viewMode}
        q={lib.q}
        sort={lib.sort}
        filterFlags={lib.filterFlags}
        filterOpen={lib.filterOpen}
        panelFilterActive={lib.panelFilterActive}
        panelFilterCount={lib.panelFilterCount}
        onQChange={lib.setQ}
        onClearSearch={() => {
          lib.setQ('')
          lib.setLibrarySearch({ q: '' })
        }}
        onToggleFilter={lib.toggleFilter}
        onToggleFlag={lib.toggleFlagFilter}
        onSortChange={(sort) => lib.setLibrarySearch({ sort })}
      />

      <div className={styles.resultsShell}>
        {lib.viewMode === 'explorer' ? (
          <LibraryExplorerView
            categories={lib.cats.data || []}
            bookmarks={lib.bookmarks.data || []}
            searchHits={lib.search.data?.hits || []}
            searching={lib.searching}
            searchQuery={lib.debouncedQ}
            folderId={lib.folderId}
            sort={lib.sort}
            showHidden={lib.showHidden}
            loading={lib.explorerLoading}
            onFolderChange={lib.onFolderChange}
            onEdit={lib.openEdit}
            onDelete={lib.setDeleteId}
            onFlags={lib.onFlags}
            filterFlags={lib.filterFlags}
            onCreate={lib.openCreate}
            canCreate={lib.canCreateInContext}
            folderReadOnly={!!lib.folderCat && !canWriteCategory(lib.folderCat)}
            viewerUserId={lib.me.data?.user.id}
            selectMode={lib.selectMode}
            selectedIds={lib.selectedIds}
            onToggleSelect={lib.toggleSelect}
            categoryMenuFor={lib.categoryMenuFor}
          />
        ) : (
          <LibraryGridView
            loading={lib.loading}
            list={lib.list}
            searching={lib.searching}
            searchQuery={lib.debouncedQ}
            emptyLibrary={lib.emptyLibrary}
            emptyFiltered={lib.emptyFiltered}
            canCreate={lib.canCreateInContext}
            viewerUserId={lib.me.data?.user.id}
            filterOpen={lib.filterOpen}
            panelFilterActive={lib.panelFilterActive}
            anyFilterActive={lib.filterActive}
            filterPanelId={FILTER_PANEL_ID}
            draftFilterCats={lib.draftFilterCats}
            draftFilterTags={lib.draftFilterTags}
            draftFilterSince={lib.draftFilterSince}
            draftVaultOnly={lib.draftVaultOnly}
            filterCats={lib.filterCats}
            filterTags={lib.filterTags}
            filterSince={lib.filterSince}
            vaultOnly={lib.vaultOnly}
            catLabel={lib.catLabel}
            categories={lib.cats.data || []}
            allTags={lib.allTags}
            onDraftCats={lib.setDraftFilterCats}
            onDraftTags={lib.setDraftFilterTags}
            onDraftSince={lib.setDraftFilterSince}
            onDraftVaultOnly={lib.setDraftVaultOnly}
            onApplyFilter={lib.applyFilter}
            onCloseFilter={() => lib.setFilterOpen(false)}
            onClearFilters={() => {
              lib.setDraftFilterCats([])
              lib.setDraftFilterTags([])
              lib.setDraftFilterSince(null)
              lib.setDraftVaultOnly(false)
              lib.setLibrarySearch({ category: [], tag: [], since: null, vault: false })
            }}
            onRemoveCat={(id) =>
              lib.setLibrarySearch({ category: lib.filterCats.filter((x) => x !== id) })
            }
            onRemoveTag={(t) =>
              lib.setLibrarySearch({ tag: lib.filterTags.filter((x) => x !== t) })
            }
            onClearSince={() => lib.setLibrarySearch({ since: null })}
            onClearVault={() => lib.setLibrarySearch({ vault: false })}
            onClearSearchAndFilters={() => {
              lib.setQ('')
              lib.setLibrarySearch({
                q: '',
                category: [],
                tag: [],
                flag: [],
                since: null,
                vault: false,
              })
            }}
            onCreate={lib.openCreate}
            onEdit={lib.openEdit}
            onDelete={lib.setDeleteId}
            onFlags={lib.onFlags}
            selectMode={lib.selectMode}
            selectedIds={lib.selectedIds}
            onToggleSelect={lib.toggleSelect}
          />
        )}
      </div>

      <LibraryPageModals
        categories={lib.cats.data || []}
        drawerOpen={lib.drawerOpen}
        onCloseDrawer={lib.closeDrawer}
        draft={lib.draft}
        bulkMode={lib.bulkMode}
        onCloseBulk={() => lib.setBulkMode(null)}
        selectedCount={lib.selectedCount}
        bulkPending={lib.bulkPending}
        onApplyCategories={(ids) => void lib.applyBulkCategories(ids)}
        onApplyTag={(tags, mode) => void lib.applyBulkTag(tags, mode)}
        dupesOpen={lib.dupesOpen}
        onCloseDupes={lib.closeDupes}
        categoryModals={lib.categoryActions.modals}
        deleteId={lib.deleteId}
        onCloseDelete={() => lib.setDeleteId(null)}
        onConfirmDelete={() => void lib.confirmDelete()}
        deletePending={lib.deleteBookmark.isPending}
        bulkDeleteOpen={lib.bulkDeleteOpen}
        onCloseBulkDelete={() => lib.setBulkDeleteOpen(false)}
        onConfirmBulkDelete={() => void lib.confirmBulkDelete()}
        bulkDeletePending={lib.bulkDelete.isPending}
        unlockOpen={lib.unlockOpen}
        onCloseUnlock={() => lib.setUnlockOpen(false)}
        vaultPass={lib.vaultPass}
        onVaultPassChange={lib.setVaultPass}
        onUnlock={() => void lib.unlockForSearch()}
        unlockPending={lib.vaultUnlock.isPending}
      />

      <Affix position={{ bottom: 24, right: 24 }}>
        <Group gap="sm">
          <Transition transition="slide-up" mounted={lib.scroll.y > 320}>
            {(transitionStyles) => (
              <Tooltip label="Back to top">
                <ActionIcon
                  style={transitionStyles}
                  size="lg"
                  radius="xl"
                  variant="filled"
                  color="violet"
                  aria-label="Back to top"
                  onClick={() => window.scrollTo({ top: 0, behavior: 'smooth' })}
                >
                  <IconArrowUp size={18} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
            )}
          </Transition>
          {lib.canCreateInContext ? (
            <Tooltip label="Add bookmark" position="left">
              <ActionIcon
                size="xl"
                radius="xl"
                variant="filled"
                color="violet"
                aria-label="Add bookmark"
                onClick={lib.openCreate}
                hiddenFrom="sm"
              >
                <IconBookmarkPlus size={22} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          ) : null}
        </Group>
      </Affix>
    </Stack>
  )
}
