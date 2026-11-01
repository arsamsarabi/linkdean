import { useEffect, useMemo, useRef, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Affix,
  Alert,
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
  IconBookmark,
  IconBookmarkPlus,
  IconCheck,
  IconCopy,
  IconFilter,
  IconFolders,
  IconLayoutGrid,
  IconLockOpen,
  IconPin,
  IconSearch,
  IconStar,
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
  useCategoryFolderActions,
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
  type SinceKey,
  type SortKey,
} from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import {
  FOLDER_ALL,
  FOLDER_UNCATEGORIZED,
  collectTagOptions,
  draftFromBookmark,
  emptyDraft,
  filterAndSortLibraryItems,
  mergeSearchWithLocalCatchUp,
  normalizeFolderId,
  sinceFilterToIso,
  toggleId,
  writableIds,
  type ExplorerFolderId,
  type FlagFilter,
  type LibraryListItem,
  type SinceFilter,
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
    since?: SinceKey
    vault?: '1'
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
  const filterSince: SinceFilter | null = searchParams.since ?? null
  const vaultOnly = searchParams.vault === '1'
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
  const [draftFilterSince, setDraftFilterSince] = useState<SinceFilter | null>(null)
  const [draftVaultOnly, setDraftVaultOnly] = useState(false)

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
  const categoryActions = useCategoryFolderActions({
    onDeleted: (id) => {
      if (folderId !== id) return
      void nav({
        to: '/library',
        search: buildLibrarySearch({
          q: debouncedQ.trim() || undefined,
          sort,
          category: filterCats,
          tag: filterTags,
          flag: filterFlags,
          since: filterSince,
          vault: vaultOnly,
          folder: FOLDER_ALL,
        }),
      })
    },
  })

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
        since: filterSince,
        vault: vaultOnly,
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
    since?: SinceFilter | null
    vault?: boolean
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
        since: patch.since !== undefined ? patch.since : filterSince,
        vault: patch.vault !== undefined ? patch.vault : vaultOnly,
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

  const { list, searchCatchUpCount } = useMemo((): {
    list: LibraryListItem[]
    searchCatchUpCount: number
  } => {
    let catchUpCount = 0
    let raw: LibraryListItem[]
    if (searching) {
      const merged = mergeSearchWithLocalCatchUp(
        search.data?.hits || [],
        bookmarks.data || [],
        debouncedQ
      )
      raw = merged.items
      catchUpCount = merged.catchUpCount
    } else {
      raw = bookmarks.data || []
    }
    const flagsById = new Map((bookmarks.data || []).map((b) => [b.id, b]))
    const items = raw.map((item) => {
      const full = flagsById.get(item.id)
      if (!full) return item
      return {
        ...item,
        is_favorite: full.is_favorite,
        is_unread: full.is_unread,
        is_pinned: full.is_pinned,
        is_vault: full.is_vault,
        created_at: full.created_at,
        updated_at: full.updated_at,
        categories: full.categories,
      }
    })
    const cats =
      folderId === FOLDER_UNCATEGORIZED && !filterCats.includes(FOLDER_UNCATEGORIZED)
        ? [...filterCats, FOLDER_UNCATEGORIZED]
        : filterCats
    return {
      list: filterAndSortLibraryItems({
        items,
        filterCats: cats,
        filterTags,
        filterFlags,
        vaultOnly,
        sinceIso: sinceFilterToIso(filterSince),
        sort,
      }),
      searchCatchUpCount: catchUpCount,
    }
  }, [
    bookmarks.data,
    search.data,
    searching,
    debouncedQ,
    filterCats,
    filterTags,
    filterFlags,
    vaultOnly,
    filterSince,
    sort,
    folderId,
  ])

  function toggleFilter(): void {
    setDraftFilterCats(filterCats)
    setDraftFilterTags(filterTags)
    setDraftFilterSince(filterSince)
    setDraftVaultOnly(vaultOnly)
    setFilterOpen((o) => !o)
  }

  function applyFilter(): void {
    setLibrarySearch({
      category: draftFilterCats,
      tag: draftFilterTags,
      since: draftFilterSince,
      vault: draftVaultOnly,
    })
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
        since: filterSince,
        vault: vaultOnly,
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

  function categoryMenuFor(cat: Category) {
    const socialEnabled = cat.shared_out || cat.access === 'read' || cat.access === 'write'
    return {
      onEdit: () => categoryActions.openEdit(cat),
      onShare: () => categoryActions.openShare(cat),
      onSocial: socialEnabled ? () => categoryActions.openSocial(cat) : undefined,
      onFork: () => categoryActions.forkCategory(cat),
      onDelete: () => categoryActions.openDelete(cat.id),
      forkPending: categoryActions.forkPending,
    }
  }

  const filterActive =
    filterCats.length > 0 ||
    filterTags.length > 0 ||
    filterFlags.length > 0 ||
    !!filterSince ||
    vaultOnly
  const filterCount =
    filterCats.length + filterTags.length + filterFlags.length + (filterSince ? 1 : 0) + (vaultOnly ? 1 : 0)
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
          {/* Always reserve subtitle height so toolbar does not jump when the list empties. */}
          <Text size="sm" c="dimmed" aria-live="polite" className={styles.countLine}>
            {!loading && viewMode === 'grid'
              ? `${list.length} bookmark${list.length === 1 ? '' : 's'}${searching ? ' matched' : ''}`
              : '\u00a0'}
          </Text>
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
      {searching && searchCatchUpCount > 0 ? (
        <Alert
          color="yellow"
          title="Search index catching up"
          withCloseButton={false}
        >
          <Group justify="space-between" align="center" wrap="wrap" gap="sm">
            <Text size="sm">
              Showing {searchCatchUpCount} local match{searchCatchUpCount === 1 ? '' : 'es'} while Meilisearch
              finishes indexing.
            </Text>
            <Button
              size="compact-sm"
              variant="light"
              loading={search.isFetching}
              onClick={() => void search.refetch()}
            >
              Retry search
            </Button>
          </Group>
        </Alert>
      ) : null}

      <Stack gap="md">
        <Group justify="space-between" align="stretch" wrap="wrap" gap="sm">
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
          <Group gap="xs" wrap="nowrap" align="stretch">
            {viewMode === 'grid' ? (
              <Tooltip label="Filter by category or tag">
                <Button
                  variant={filterOpen || filterActive ? 'light' : 'default'}
                  leftSection={<IconFilter {...iconSm} />}
                  onClick={toggleFilter}
                  aria-expanded={filterOpen}
                  aria-controls={FILTER_PANEL_ID}
                  h="100%"
                >
                  Filter
                  {filterActive ? ` (${filterCount})` : ''}
                </Button>
              </Tooltip>
            ) : null}
            <Button.Group className={styles.flagFilters}>
              <Button
                variant={filterFlags.includes('unread') ? 'light' : 'default'}
                leftSection={<IconBookmark {...iconSm} />}
                onClick={() => toggleFlagFilter('unread')}
                h="100%"
              >
                Reading list
              </Button>
              <Button
                variant={filterFlags.includes('favorite') ? 'light' : 'default'}
                leftSection={<IconStar {...iconSm} />}
                onClick={() => toggleFlagFilter('favorite')}
                h="100%"
              >
                Favorites
              </Button>
              <Button
                variant={filterFlags.includes('pinned') ? 'light' : 'default'}
                leftSection={<IconPin {...iconSm} />}
                onClick={() => toggleFlagFilter('pinned')}
                h="100%"
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

      <div className={styles.resultsShell}>
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
          categoryMenuFor={categoryMenuFor}
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
          draftFilterSince={draftFilterSince}
          draftVaultOnly={draftVaultOnly}
          filterCats={filterCats}
          filterTags={filterTags}
          filterFlags={filterFlags}
          filterSince={filterSince}
          vaultOnly={vaultOnly}
          catLabel={catLabel}
          categories={cats.data || []}
          allTags={allTags}
          onDraftCats={setDraftFilterCats}
          onDraftTags={setDraftFilterTags}
          onDraftSince={setDraftFilterSince}
          onDraftVaultOnly={setDraftVaultOnly}
          onApplyFilter={applyFilter}
          onCloseFilter={() => setFilterOpen(false)}
          onClearFilters={() =>
            setLibrarySearch({ category: [], tag: [], flag: [], since: null, vault: false })
          }
          onRemoveCat={(id) => setLibrarySearch({ category: filterCats.filter((x) => x !== id) })}
          onRemoveTag={(t) => setLibrarySearch({ tag: filterTags.filter((x) => x !== t) })}
          onRemoveFlag={(f) => setLibrarySearch({ flag: filterFlags.filter((x) => x !== f) })}
          onClearSince={() => setLibrarySearch({ since: null })}
          onClearVault={() => setLibrarySearch({ vault: false })}
          onClearSearchAndFilters={() => {
            setQ('')
            setLibrarySearch({ q: '', category: [], tag: [], flag: [], since: null, vault: false })
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
      </div>

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

      {categoryActions.modals}

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
