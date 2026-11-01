import { useEffect, useMemo, useRef, useState } from 'react'
import { useDebouncedValue, useDisclosure, useHotkeys, useWindowScroll } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import { useNavigate, useSearch } from '@tanstack/react-router'
import type { BookmarkDraft, BulkEditMode } from '@/components'
import { useMe } from '@/hooks/auth'
import {
  useBookmarks,
  useBulkAddCategories,
  useBulkDeleteBookmarks,
  useBulkTagBookmarks,
  useDeleteBookmark,
  useSearchBookmarks,
  useUpdateBookmark,
} from '@/hooks/bookmarks'
import { useCategories, useCategoryFolderActions } from '@/hooks/categories'
import { useUpdateSettings } from '@/hooks/settings'
import { useVaultUnlock } from '@/hooks/vault'
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
  type SinceFilter,
} from '@/services'
import { useUi } from '@/stores'

export type LibraryViewMode = 'grid' | 'explorer'

export function useLibraryPage() {
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

  // Panel filters only (not Favorites / Reading list / Pinned toggles).
  const panelFilterActive =
    filterCats.length > 0 || filterTags.length > 0 || !!filterSince || vaultOnly
  const panelFilterCount =
    filterCats.length + filterTags.length + (filterSince ? 1 : 0) + (vaultOnly ? 1 : 0)
  const filterActive = panelFilterActive || filterFlags.length > 0
  const loading = bookmarks.isLoading || (searching && search.isFetching && !search.data)
  const emptyLibrary = !loading && !searching && !filterActive && list.length === 0
  const emptyFiltered = !loading && list.length === 0 && (searching || filterActive)
  const explorerLoading = bookmarks.isLoading || cats.isLoading
  const selectableIds = useMemo(() => writableIds(list), [list])
  const selectedCount = selectedIds.size
  const allVisibleSelected =
    selectableIds.length > 0 && selectableIds.every((id) => selectedIds.has(id))

  return {
    me,
    showHidden,
    searchRef,
    scroll,
    filterCats,
    filterTags,
    filterFlags,
    filterSince,
    vaultOnly,
    sort,
    folderId,
    viewMode,
    onViewModeChange,
    q,
    setQ,
    debouncedQ,
    filterOpen,
    setFilterOpen,
    draftFilterCats,
    setDraftFilterCats,
    draftFilterTags,
    setDraftFilterTags,
    draftFilterSince,
    setDraftFilterSince,
    draftVaultOnly,
    setDraftVaultOnly,
    drawerOpen,
    closeDrawer,
    draft,
    unlockOpen,
    setUnlockOpen,
    vaultPass,
    setVaultPass,
    deleteId,
    setDeleteId,
    selectMode,
    selectedIds,
    setSelectedIds,
    bulkMode,
    setBulkMode,
    bulkDeleteOpen,
    setBulkDeleteOpen,
    dupesOpen,
    openDupes,
    closeDupes,
    cats,
    bookmarks,
    searching,
    search,
    deleteBookmark,
    bulkDelete,
    vaultUnlock,
    bulkPending,
    categoryActions,
    setLibrarySearch,
    toggleFlagFilter,
    onFlags,
    allTags,
    catLabel,
    list,
    searchCatchUpCount,
    toggleFilter,
    applyFilter,
    folderCat,
    canCreateInContext,
    openCreate,
    exitSelectMode,
    enterSelectMode,
    toggleSelect,
    openEdit,
    unlockForSearch,
    confirmDelete,
    applyBulkCategories,
    applyBulkTag,
    confirmBulkDelete,
    onFolderChange,
    categoryMenuFor,
    panelFilterActive,
    panelFilterCount,
    filterActive,
    loading,
    emptyLibrary,
    emptyFiltered,
    explorerLoading,
    selectableIds,
    selectedCount,
    allVisibleSelected,
  }
}
