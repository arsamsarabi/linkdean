import { useEffect, useMemo, useRef, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Affix,
  Button,
  Center,
  Group,
  Kbd,
  Modal,
  PasswordInput,
  SegmentedControl,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
  Tooltip,
  Transition,
} from '@mantine/core'
import { useDebouncedValue, useDisclosure, useHotkeys, useWindowScroll } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import {
  IconArrowUp,
  IconBookmarkPlus,
  IconCheck,
  IconCopy,
  IconFilter,
  IconFolders,
  IconLayoutGrid,
  IconLockOpen,
  IconSearch,
  IconTags,
  IconTrash,
  IconX,
} from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import {
  BookmarkDrawer,
  BulkEditDrawer,
  ConfirmModal,
  DuplicateMergeDrawer,
  LibraryExplorerView,
  QueryErrorAlert,
  type BookmarkDraft,
  type BulkEditMode,
} from '@/components'
import {
  useBookmarks,
  useBulkAddCategories,
  useBulkDeleteBookmarks,
  useBulkTagBookmarks,
  useCategories,
  useDeleteBookmark,
  useMe,
  useSearchBookmarks,
  useUpdateBookmark,
  useUpdateSettings,
  useVaultUnlock,
} from '@/hooks'
import {
  asStringList,
  buildLibrarySearch,
  canWriteCategory,
  categoryPath,
  type FlagKey,
  type SortKey,
} from '@/lib'
import type { Bookmark } from '@/schemas'
import {
  FOLDER_ALL,
  FOLDER_UNCATEGORIZED,
  collectTagOptions,
  draftFromBookmark,
  emptyDraft,
  filterAndSortLibraryItems,
  normalizeFolderId,
  toggleId,
  writableIds,
  type ExplorerFolderId,
  type FlagFilter,
  type LibraryListItem,
} from '@/services'
import { useUi } from '@/stores'
import { LibraryGridView } from './LibraryGridView'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const
const FILTER_PANEL_ID = 'library-filter-panel'

type LibraryViewMode = 'grid' | 'explorer'

export function LibraryPage(): ReactElement {
  const nav = useNavigate()
  const searchParams = useSearch({ strict: false }) as {
    q?: string
    sort?: SortKey
    category?: string | string[]
    tag?: string | string[]
    flag?: string | string[]
    folder?: string
    create?: true
    focus?: 'search'
  }

  const me = useMe()
  const updateSettings = useUpdateSettings()
  const showHidden = useUi((s) => s.showHidden)
  const searchRef = useRef<HTMLInputElement>(null)
  const [scroll] = useWindowScroll()

  const filterCats = useMemo(() => asStringList(searchParams.category), [searchParams.category])
  const filterTags = useMemo(() => asStringList(searchParams.tag), [searchParams.tag])
  const filterFlags = useMemo(
    () => asStringList(searchParams.flag).filter((f): f is FlagFilter => ['unread', 'favorite', 'pinned'].includes(f)),
    [searchParams.flag]
  )
  const sort: SortKey = searchParams.sort ?? 'updated_desc'
  const urlQ = searchParams.q ?? ''
  const folderId = normalizeFolderId(searchParams.folder)

  const settingsView: LibraryViewMode = me.data?.settings.library_view === 'explorer' ? 'explorer' : 'grid'
  const [viewMode, setViewMode] = useState<LibraryViewMode>(settingsView)
  const [viewFromSettings, setViewFromSettings] = useState<LibraryViewMode | null>(null)
  if (me.data && viewFromSettings !== settingsView) {
    setViewFromSettings(settingsView)
    setViewMode(settingsView)
  }

  function onViewModeChange(next: LibraryViewMode): void {
    setViewMode(next)
    if (next === settingsView || updateSettings.isPending) return
    updateSettings.mutate(
      { library_view: next },
      {
        onError: () => {
          setViewMode(settingsView)
          notifications.show({ color: 'red', message: 'Could not save view preference' })
        },
      }
    )
  }

  const [q, setQ] = useState(urlQ)
  const [qFromUrl, setQFromUrl] = useState(urlQ)
  if (qFromUrl !== urlQ) {
    setQFromUrl(urlQ)
    setQ(urlQ)
  }
  const [debouncedQ] = useDebouncedValue(q, 280)
  const [filterOpen, setFilterOpen] = useState(false)
  const [draftFilterCats, setDraftFilterCats] = useState<string[]>([])
  const [draftFilterTags, setDraftFilterTags] = useState<string[]>([])

  const [drawerOpen, { open: openDrawer, close: closeDrawer }] = useDisclosure(false)
  const [draft, setDraft] = useState<BookmarkDraft | null>(null)
  const [unlockOpen, setUnlockOpen] = useState(false)
  const [vaultPass, setVaultPass] = useState('')
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [selectMode, setSelectMode] = useState(false)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [bulkMode, setBulkMode] = useState<BulkEditMode | null>(null)
  const [bulkDeleteOpen, setBulkDeleteOpen] = useState(false)
  const [dupesOpen, { open: openDupes, close: closeDupes }] = useDisclosure(false)

  const cats = useCategories()
  const bookmarks = useBookmarks()
  const searching = debouncedQ.trim().length > 1
  const search = useSearchBookmarks(debouncedQ, searching)
  const deleteBookmark = useDeleteBookmark()
  const updateBookmark = useUpdateBookmark()
  const bulkCategories = useBulkAddCategories()
  const bulkTag = useBulkTagBookmarks()
  const bulkDelete = useBulkDeleteBookmarks()
  const vaultUnlock = useVaultUnlock()
  const bulkPending = bulkCategories.isPending || bulkTag.isPending || bulkDelete.isPending

  useEffect(() => {
    const next = debouncedQ.trim() || undefined
    const cur = searchParams.q || undefined
    if (next === cur) return
    void nav({
      to: '/library',
      search: buildLibrarySearch({
        q: next,
        sort,
        category: filterCats,
        tag: filterTags,
        flag: filterFlags,
        folder: folderId,
      }),
      replace: true,
    })
  }, [debouncedQ]) // oxlint-disable-line react-hooks/exhaustive-deps -- sync debounce → URL only

  function setLibrarySearch(patch: {
    q?: string
    sort?: SortKey
    category?: string[]
    tag?: string[]
    flag?: string[]
    /** Pass `all` or empty string to clear folder selection. */
    folder?: string
  }): void {
    void nav({
      to: '/library',
      search: buildLibrarySearch({
        q: patch.q !== undefined ? patch.q : q,
        sort: patch.sort ?? sort,
        category: patch.category ?? filterCats,
        tag: patch.tag ?? filterTags,
        flag: patch.flag ?? filterFlags,
        folder: patch.folder !== undefined ? patch.folder || FOLDER_ALL : folderId,
      }),
    })
  }

  function toggleFlagFilter(key: FlagKey): void {
    const next = filterFlags.includes(key)
      ? filterFlags.filter((f) => f !== key)
      : [...filterFlags, key]
    setLibrarySearch({ flag: next })
  }

  function onFlags(
    id: string,
    patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>
  ): void {
    updateBookmark.mutate(
      { id, body: patch },
      {
        onError: (e) => notifications.show({ color: 'red', message: String(e) }),
      }
    )
  }

  useEffect(() => {
    if (!search.data?.vault_prompt) return
    // oxlint-disable-next-line react/set-state-in-effect -- open modal from search vault_prompt flag
    setUnlockOpen(true)
  }, [search.data?.vault_prompt])

  const allTags = useMemo(() => collectTagOptions(bookmarks.data || []), [bookmarks.data])

  const catLabel = useMemo(() => {
    const list = cats.data || []
    const byId = new Map(list.map((c) => [c.id, c]))
    const map = new Map<string, string>()
    map.set(FOLDER_UNCATEGORIZED, 'Unsorted')
    for (const c of list) map.set(c.id, categoryPath(c, byId))
    return map
  }, [cats.data])

  const list = useMemo((): LibraryListItem[] => {
    const raw: LibraryListItem[] = searching ? search.data?.hits || [] : bookmarks.data || []
    const flagsById = new Map((bookmarks.data || []).map((b) => [b.id, b]))
    const items = raw.map((item) => {
      const full = flagsById.get(item.id)
      if (!full) return item
      return {
        ...item,
        is_favorite: full.is_favorite,
        is_unread: full.is_unread,
        is_pinned: full.is_pinned,
      }
    })
    const cats =
      folderId === FOLDER_UNCATEGORIZED && !filterCats.includes(FOLDER_UNCATEGORIZED)
        ? [...filterCats, FOLDER_UNCATEGORIZED]
        : filterCats
    return filterAndSortLibraryItems({
      items,
      filterCats: cats,
      filterTags,
      filterFlags,
      sort,
    })
  }, [bookmarks.data, search.data, searching, filterCats, filterTags, filterFlags, sort, folderId])

  function toggleFilter(): void {
    setDraftFilterCats(filterCats)
    setDraftFilterTags(filterTags)
    setFilterOpen((o) => !o)
  }

  function applyFilter(): void {
    setLibrarySearch({ category: draftFilterCats, tag: draftFilterTags })
    setFilterOpen(false)
  }

  const folderCat = useMemo(() => {
    if (!folderId || folderId === FOLDER_ALL || folderId === FOLDER_UNCATEGORIZED) return null
    return (cats.data || []).find((c) => c.id === folderId) ?? null
  }, [cats.data, folderId])

  /** Create allowed at All / uncategorized; gated when browsing a specific shared folder. */
  const canCreateInContext = !folderCat || canWriteCategory(folderCat)

  function openCreate(): void {
    if (!canCreateInContext) return
    if (folderCat && canWriteCategory(folderCat)) {
      setDraft({ ...emptyDraft(), category_ids: [folderCat.id] })
    } else {
      setDraft(null)
    }
    openDrawer()
  }

  function exitSelectMode(): void {
    setSelectMode(false)
    setSelectedIds(new Set())
    setBulkMode(null)
    setBulkDeleteOpen(false)
  }

  function enterSelectMode(): void {
    setSelectMode(true)
    setSelectedIds(new Set())
  }

  function toggleSelect(id: string): void {
    setSelectedIds((prev) => toggleId(prev, id))
  }

  useHotkeys([
    [
      'n',
      (e) => {
        const t = e.target as HTMLElement | null
        if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable) return
        if (!canCreateInContext) return
        e.preventDefault()
        openCreate()
      },
    ],
    [
      'Escape',
      () => {
        if (selectMode) exitSelectMode()
      },
    ],
  ])

  /** Consume one-shot command-palette flags (`create` / `focus`) then strip from URL. */
  useEffect(() => {
    if (!searchParams.create && searchParams.focus !== 'search') return
    if (searchParams.create) openCreate()
    if (searchParams.focus === 'search') {
      window.requestAnimationFrame(() => searchRef.current?.focus())
    }
    void nav({
      to: '/library',
      search: buildLibrarySearch({
        q: urlQ,
        sort,
        category: filterCats,
        tag: filterTags,
        flag: filterFlags,
        folder: folderId,
      }),
      replace: true,
    })
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- one-shot URL flags only
  }, [searchParams.create, searchParams.focus])

  function openEdit(id: string): void {
    const fromList = (bookmarks.data || []).find((x) => x.id === id)
    const fromSearch = (search.data?.hits || []).find((x) => x.id === id)
    const b = fromList || fromSearch
    if (!b) {
      notifications.show({ color: 'red', message: 'Bookmark not found' })
      return
    }
    if ('can_write' in b && b.can_write === false) {
      notifications.show({ color: 'yellow', message: 'Read-only shared bookmark' })
      return
    }
    if (fromList) {
      setDraft(draftFromBookmark(fromList))
      openDrawer()
      return
    }
    notifications.show({ color: 'red', message: 'Open the bookmark from the library to edit' })
  }

  async function unlockForSearch(): Promise<void> {
    try {
      await vaultUnlock.mutateAsync(vaultPass)
      setVaultPass('')
      setUnlockOpen(false)
      notifications.show({ message: 'Vault unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function confirmDelete(): Promise<void> {
    if (!deleteId) return
    try {
      await deleteBookmark.mutateAsync(deleteId)
      notifications.show({ message: 'Bookmark deleted' })
      setDeleteId(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  function notifyBulk(label: string, result: { ok: number; failed: number }): void {
    if (result.failed === 0) {
      notifications.show({ message: `${label}: ${result.ok} bookmark${result.ok === 1 ? '' : 's'}` })
      return
    }
    notifications.show({
      color: 'yellow',
      message: `${label}: ${result.ok} ok, ${result.failed} failed`,
    })
  }

  async function applyBulkCategories(categoryIds: string[]): Promise<void> {
    const ids = [...selectedIds]
    if (ids.length === 0) return
    const byId = new Map<string, Bookmark>((bookmarks.data || []).map((b) => [b.id, b]))
    const result = await bulkCategories.mutateAsync({ ids, category_ids: categoryIds, byId })
    notifyBulk('Categories added', result)
    setBulkMode(null)
    exitSelectMode()
  }

  async function applyBulkTag(tags: string[], tagMode: 'add' | 'replace'): Promise<void> {
    const ids = [...selectedIds]
    if (ids.length === 0) return
    const byId = new Map<string, Bookmark>((bookmarks.data || []).map((b) => [b.id, b]))
    const result = await bulkTag.mutateAsync({ ids, tags, mode: tagMode, byId })
    notifyBulk(tagMode === 'add' ? 'Tagged' : 'Tags replaced', result)
    setBulkMode(null)
    exitSelectMode()
  }

  async function confirmBulkDelete(): Promise<void> {
    const ids = [...selectedIds]
    if (ids.length === 0) return
    const result = await bulkDelete.mutateAsync({ ids })
    notifyBulk('Deleted', result)
    setBulkDeleteOpen(false)
    exitSelectMode()
  }

  function onFolderChange(id: ExplorerFolderId): void {
    setLibrarySearch({ folder: id })
  }

  const filterActive = filterCats.length > 0 || filterTags.length > 0 || filterFlags.length > 0
  const loading = bookmarks.isLoading || (searching && search.isFetching && !search.data)
  const emptyLibrary = !loading && !searching && !filterActive && list.length === 0
  const emptyFiltered = !loading && list.length === 0 && (searching || filterActive)
  const explorerLoading = bookmarks.isLoading || cats.isLoading
  const selectableIds = useMemo(() => writableIds(list), [list])
  const selectedCount = selectedIds.size
  const allVisibleSelected =
    selectableIds.length > 0 && selectableIds.every((id) => selectedIds.has(id))

  return (
    <Stack gap="xl">
      <Group justify="space-between" align="flex-end" wrap="wrap" gap="md">
        <Stack gap={4}>
          <Title order={2}>Library</Title>
          {!loading && viewMode === 'grid' && list.length > 0 ? (
            <Text size="sm" c="dimmed" aria-live="polite">
              {list.length} bookmark{list.length === 1 ? '' : 's'}
              {searching ? ' matched' : ''}
            </Text>
          ) : null}
        </Stack>
        <Group gap="sm" wrap="wrap">
          <SegmentedControl
            aria-label="Library view"
            value={viewMode}
            onChange={(v) => onViewModeChange(v as LibraryViewMode)}
            data={[
              {
                value: 'grid',
                label: (
                  <Center style={{ gap: 6 }}>
                    <IconLayoutGrid size={16} stroke={1.5} />
                    <span>Grid</span>
                  </Center>
                ),
              },
              {
                value: 'explorer',
                label: (
                  <Center style={{ gap: 6 }}>
                    <IconFolders size={16} stroke={1.5} />
                    <span>Explorer</span>
                  </Center>
                ),
              },
            ]}
          />
          {selectMode ? (
            <Button variant="default" onClick={exitSelectMode}>
              Cancel select
            </Button>
          ) : (
            <>
              <Button
                variant="default"
                leftSection={<IconCheck {...iconSm} />}
                onClick={enterSelectMode}
                disabled={loading || list.length === 0}
              >
                Select
              </Button>
              <Tooltip label="Find and merge duplicate URLs">
                <Button variant="default" leftSection={<IconCopy {...iconSm} />} onClick={openDupes}>
                  Duplicates
                </Button>
              </Tooltip>
            </>
          )}
          {canCreateInContext ? (
            <Tooltip label="New bookmark (N)">
              <Button leftSection={<IconBookmarkPlus {...iconSm} />} onClick={openCreate}>
                Add bookmark
              </Button>
            </Tooltip>
          ) : (
            <Tooltip label="Read-only shared folder">
              <Button leftSection={<IconBookmarkPlus {...iconSm} />} disabled>
                Add bookmark
              </Button>
            </Tooltip>
          )}
        </Group>
      </Group>

      {selectMode ? (
        <Group justify="space-between" wrap="wrap" gap="sm" p="sm" className={styles.filterPanel}>
          <Group gap="sm">
            <Text size="sm" fw={600}>
              {selectedCount} selected
            </Text>
            <Button
              size="compact-sm"
              variant="subtle"
              onClick={() =>
                setSelectedIds(allVisibleSelected ? new Set() : new Set(selectableIds))
              }
              disabled={selectableIds.length === 0}
            >
              {allVisibleSelected ? 'Clear all' : 'Select all'}
            </Button>
          </Group>
          <Group gap="xs">
            <Button
              size="compact-sm"
              variant="light"
              leftSection={<IconFolders size={14} stroke={1.5} />}
              disabled={selectedCount === 0}
              onClick={() => setBulkMode('categories')}
            >
              Categories
            </Button>
            <Button
              size="compact-sm"
              variant="light"
              leftSection={<IconTags size={14} stroke={1.5} />}
              disabled={selectedCount === 0}
              onClick={() => setBulkMode('tag')}
            >
              Tag
            </Button>
            <Button
              size="compact-sm"
              color="red"
              variant="light"
              leftSection={<IconTrash size={14} stroke={1.5} />}
              disabled={selectedCount === 0}
              onClick={() => setBulkDeleteOpen(true)}
            >
              Delete
            </Button>
          </Group>
        </Group>
      ) : null}

      {cats.isError ? (
        <QueryErrorAlert error={cats.error} onRetry={() => void cats.refetch()} title="Categories failed" />
      ) : null}
      {bookmarks.isError && !searching ? (
        <QueryErrorAlert error={bookmarks.error} onRetry={() => void bookmarks.refetch()} title="Bookmarks failed" />
      ) : null}
      {search.isError && searching ? (
        <QueryErrorAlert error={search.error} onRetry={() => void search.refetch()} title="Search failed" />
      ) : null}

      <Stack gap="md">
        <Group justify="space-between" align="center" wrap="wrap" gap="sm">
          <TextInput
            ref={searchRef}
            placeholder="Search bookmarks…"
            leftSection={<IconSearch {...iconSm} />}
            rightSection={
              q ? (
                <ActionIcon
                  variant="subtle"
                  color="gray"
                  aria-label="Clear search"
                  onClick={() => {
                    setQ('')
                    setLibrarySearch({ q: '' })
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
            rightSectionPointerEvents={q ? 'all' : 'none'}
            value={q}
            onChange={(e) => setQ(e.currentTarget.value)}
            className={styles.searchInput}
          />
          <Group gap="xs" wrap="nowrap">
            {viewMode === 'grid' ? (
              <Tooltip label="Filter by category or tag">
                <Button
                  variant={filterOpen || filterActive ? 'light' : 'default'}
                  leftSection={<IconFilter {...iconSm} />}
                  onClick={toggleFilter}
                  aria-expanded={filterOpen}
                  aria-controls={FILTER_PANEL_ID}
                >
                  Filter
                  {filterActive
                    ? ` (${filterCats.length + filterTags.length + filterFlags.length})`
                    : ''}
                </Button>
              </Tooltip>
            ) : null}
            <Button.Group>
              <Button
                size="compact-sm"
                variant={filterFlags.includes('unread') ? 'light' : 'default'}
                onClick={() => toggleFlagFilter('unread')}
              >
                Reading list
              </Button>
              <Button
                size="compact-sm"
                variant={filterFlags.includes('favorite') ? 'light' : 'default'}
                onClick={() => toggleFlagFilter('favorite')}
              >
                Favorites
              </Button>
              <Button
                size="compact-sm"
                variant={filterFlags.includes('pinned') ? 'light' : 'default'}
                onClick={() => toggleFlagFilter('pinned')}
              >
                Pinned
              </Button>
            </Button.Group>
            <Select
              aria-label="Sort"
              value={sort}
              onChange={(v) => v && setLibrarySearch({ sort: v as SortKey })}
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

      {viewMode === 'explorer' ? (
        <LibraryExplorerView
          categories={cats.data || []}
          bookmarks={bookmarks.data || []}
          searchHits={search.data?.hits || []}
          searching={searching}
          searchQuery={debouncedQ}
          folderId={folderId}
          sort={sort}
          showHidden={showHidden}
          loading={explorerLoading}
          onFolderChange={onFolderChange}
          onEdit={openEdit}
          onDelete={setDeleteId}
          onFlags={onFlags}
          filterFlags={filterFlags}
          onCreate={openCreate}
          canCreate={canCreateInContext}
          folderReadOnly={!!folderCat && !canWriteCategory(folderCat)}
          viewerUserId={me.data?.user.id}
          selectMode={selectMode}
          selectedIds={selectedIds}
          onToggleSelect={toggleSelect}
        />
      ) : (
        <LibraryGridView
          loading={loading}
          list={list}
          searching={searching}
          searchQuery={debouncedQ}
          emptyLibrary={emptyLibrary}
          emptyFiltered={emptyFiltered}
          canCreate={canCreateInContext}
          viewerUserId={me.data?.user.id}
          filterOpen={filterOpen}
          filterActive={filterActive}
          filterPanelId={FILTER_PANEL_ID}
          draftFilterCats={draftFilterCats}
          draftFilterTags={draftFilterTags}
          filterCats={filterCats}
          filterTags={filterTags}
          filterFlags={filterFlags}
          catLabel={catLabel}
          categories={cats.data || []}
          allTags={allTags}
          onDraftCats={setDraftFilterCats}
          onDraftTags={setDraftFilterTags}
          onApplyFilter={applyFilter}
          onCloseFilter={() => setFilterOpen(false)}
          onClearFilters={() => setLibrarySearch({ category: [], tag: [], flag: [] })}
          onRemoveCat={(id) => setLibrarySearch({ category: filterCats.filter((x) => x !== id) })}
          onRemoveTag={(t) => setLibrarySearch({ tag: filterTags.filter((x) => x !== t) })}
          onRemoveFlag={(f) => setLibrarySearch({ flag: filterFlags.filter((x) => x !== f) })}
          onClearSearchAndFilters={() => {
            setQ('')
            setLibrarySearch({ q: '', category: [], tag: [], flag: [] })
          }}
          onCreate={openCreate}
          onEdit={openEdit}
          onDelete={setDeleteId}
          onFlags={onFlags}
          selectMode={selectMode}
          selectedIds={selectedIds}
          onToggleSelect={toggleSelect}
        />
      )}

      <BookmarkDrawer opened={drawerOpen} onClose={closeDrawer} categories={cats.data || []} initial={draft} />

      <BulkEditDrawer
        opened={bulkMode != null}
        onClose={() => setBulkMode(null)}
        mode={bulkMode ?? 'categories'}
        count={selectedCount}
        categories={cats.data || []}
        pending={bulkPending}
        onApplyCategories={(ids) => void applyBulkCategories(ids)}
        onApplyTag={(tags, mode) => void applyBulkTag(tags, mode)}
      />

      <DuplicateMergeDrawer opened={dupesOpen} onClose={closeDupes} />

      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        onConfirm={() => void confirmDelete()}
        title="Delete bookmark?"
        body="Moves this bookmark to Deleted items. You can restore it later."
        confirmLabel="Delete"
        loading={deleteBookmark.isPending}
      />

      <ConfirmModal
        opened={bulkDeleteOpen}
        onClose={() => setBulkDeleteOpen(false)}
        onConfirm={() => void confirmBulkDelete()}
        title={`Delete ${selectedCount} bookmark${selectedCount === 1 ? '' : 's'}?`}
        body="Moves selected bookmarks to Deleted items. You can restore them later."
        confirmLabel="Delete"
        loading={bulkDelete.isPending}
      />

      <Modal opened={unlockOpen} onClose={() => setUnlockOpen(false)} title="Unlock vault for search">
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            Config options say vault should be included in search. Unlock to include vault hits.
          </Text>
          <PasswordInput
            label="Vault passphrase"
            value={vaultPass}
            onChange={(e) => setVaultPass(e.currentTarget.value)}
          />
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setUnlockOpen(false)}>
              Skip
            </Button>
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              onClick={() => void unlockForSearch()}
              disabled={vaultPass.length < 8}
              loading={vaultUnlock.isPending}
            >
              Unlock
            </Button>
          </Group>
        </Stack>
      </Modal>

      <Affix position={{ bottom: 24, right: 24 }}>
        <Group gap="sm">
          <Transition transition="slide-up" mounted={scroll.y > 320}>
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
          {canCreateInContext ? (
            <Tooltip label="Add bookmark" position="left">
              <ActionIcon
                size="xl"
                radius="xl"
                variant="filled"
                color="violet"
                aria-label="Add bookmark"
                onClick={openCreate}
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
