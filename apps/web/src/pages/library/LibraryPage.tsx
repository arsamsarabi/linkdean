import { type ReactElement } from 'react'
import {
  ActionIcon,
  Affix,
  Alert,
  Button,
  Group,
  Kbd,
  Modal,
  PasswordInput,
  Select,
  Stack,
  Text,
  TextInput,
  Tooltip,
  Transition,
} from '@mantine/core'
import {
  IconArrowUp,
  IconBookmark,
  IconBookmarkPlus,
  IconFilter,
  IconLockOpen,
  IconPin,
  IconSearch,
  IconStar,
  IconX,
} from '@tabler/icons-react'
import {
  BookmarkDrawer,
  BulkEditDrawer,
  ConfirmModal,
  DuplicateMergeDrawer,
  LibraryExplorerView,
  QueryErrorAlert,
} from '@/components'
import { useLibraryPage } from '@/hooks'
import { canWriteCategory, type SortKey } from '@/lib'
import { LibraryBulkBar } from './LibraryBulkBar'
import { LibraryGridView } from './LibraryGridView'
import { LibraryPageHeader } from './LibraryPageHeader'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const
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

      <Stack gap="md">
        <Group justify="space-between" align="stretch" wrap="wrap" gap="sm">
          <TextInput
            ref={lib.searchRef}
            placeholder="Search bookmarks…"
            leftSection={<IconSearch {...iconSm} />}
            rightSection={
              lib.q ? (
                <ActionIcon
                  variant="subtle"
                  color="gray"
                  aria-label="Clear search"
                  onClick={() => {
                    lib.setQ('')
                    lib.setLibrarySearch({ q: '' })
                  }}
                >
                  <IconX size={14} stroke={1.5} />
                </ActionIcon>
              ) : (
                <Kbd visibleFrom="sm" size="xs">
                  ⌘K
                </Kbd>
              )
            }
            rightSectionPointerEvents={lib.q ? 'all' : 'none'}
            value={lib.q}
            onChange={(e) => lib.setQ(e.currentTarget.value)}
            className={styles.searchInput}
          />
          <Group gap="xs" wrap="nowrap" align="stretch">
            {lib.viewMode === 'grid' ? (
              <Tooltip label="Filter by category or tag">
                <Button
                  variant={lib.filterOpen || lib.panelFilterActive ? 'light' : 'default'}
                  leftSection={<IconFilter {...iconSm} />}
                  onClick={lib.toggleFilter}
                  aria-expanded={lib.filterOpen}
                  aria-controls={FILTER_PANEL_ID}
                  h="100%"
                >
                  Filter
                  {lib.panelFilterActive ? ` (${lib.panelFilterCount})` : ''}
                </Button>
              </Tooltip>
            ) : null}
            <Button.Group className={styles.flagFilters}>
              <Button
                variant={lib.filterFlags.includes('unread') ? 'light' : 'default'}
                leftSection={<IconBookmark {...iconSm} />}
                onClick={() => lib.toggleFlagFilter('unread')}
                h="100%"
              >
                Reading list
              </Button>
              <Button
                variant={lib.filterFlags.includes('favorite') ? 'light' : 'default'}
                leftSection={<IconStar {...iconSm} />}
                onClick={() => lib.toggleFlagFilter('favorite')}
                h="100%"
              >
                Favorites
              </Button>
              <Button
                variant={lib.filterFlags.includes('pinned') ? 'light' : 'default'}
                leftSection={<IconPin {...iconSm} />}
                onClick={() => lib.toggleFlagFilter('pinned')}
                h="100%"
              >
                Pinned
              </Button>
            </Button.Group>
            <Select
              aria-label="Sort"
              value={lib.sort}
              onChange={(v) => v && lib.setLibrarySearch({ sort: v as SortKey })}
              allowDeselect={false}
              w={200}
              data={[
                { value: 'updated_desc', label: 'Recently updated' },
                { value: 'updated_asc', label: 'Oldest updated' },
                { value: 'created_desc', label: 'Newest created' },
                { value: 'title_asc', label: 'Title A–Z' },
                { value: 'title_desc', label: 'Title Z–A' },
              ]}
            />
          </Group>
        </Group>
      </Stack>

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

      <BookmarkDrawer
        opened={lib.drawerOpen}
        onClose={lib.closeDrawer}
        categories={lib.cats.data || []}
        initial={lib.draft}
      />

      <BulkEditDrawer
        opened={lib.bulkMode != null}
        onClose={() => lib.setBulkMode(null)}
        mode={lib.bulkMode ?? 'categories'}
        count={lib.selectedCount}
        categories={lib.cats.data || []}
        pending={lib.bulkPending}
        onApplyCategories={(ids) => void lib.applyBulkCategories(ids)}
        onApplyTag={(tags, mode) => void lib.applyBulkTag(tags, mode)}
      />

      <DuplicateMergeDrawer opened={lib.dupesOpen} onClose={lib.closeDupes} />

      {lib.categoryActions.modals}

      <ConfirmModal
        opened={!!lib.deleteId}
        onClose={() => lib.setDeleteId(null)}
        onConfirm={() => void lib.confirmDelete()}
        title="Delete bookmark?"
        body="Moves this bookmark to Deleted items. You can restore it later."
        confirmLabel="Delete"
        loading={lib.deleteBookmark.isPending}
      />

      <ConfirmModal
        opened={lib.bulkDeleteOpen}
        onClose={() => lib.setBulkDeleteOpen(false)}
        onConfirm={() => void lib.confirmBulkDelete()}
        title={`Delete ${lib.selectedCount} bookmark${lib.selectedCount === 1 ? '' : 's'}?`}
        body="Moves selected bookmarks to Deleted items. You can restore them later."
        confirmLabel="Delete"
        loading={lib.bulkDelete.isPending}
      />

      <Modal opened={lib.unlockOpen} onClose={() => lib.setUnlockOpen(false)} title="Unlock vault for search">
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            Config options say vault should be included in search. Unlock to include vault hits.
          </Text>
          <PasswordInput
            label="Vault passphrase"
            value={lib.vaultPass}
            onChange={(e) => lib.setVaultPass(e.currentTarget.value)}
          />
          <Group justify="flex-end">
            <Button variant="default" onClick={() => lib.setUnlockOpen(false)}>
              Skip
            </Button>
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              onClick={() => void lib.unlockForSearch()}
              disabled={lib.vaultPass.length < 8}
              loading={lib.vaultUnlock.isPending}
            >
              Unlock
            </Button>
          </Group>
        </Stack>
      </Modal>

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
