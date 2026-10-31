import { useEffect, useMemo, useRef, useState, type ReactElement } from 'react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import {
  ActionIcon,
  Affix,
  Button,
  Collapse,
  Group,
  Kbd,
  Modal,
  MultiSelect,
  PasswordInput,
  Pill,
  Select,
  SimpleGrid,
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
  IconFilter,
  IconLockOpen,
  IconSearch,
  IconX,
} from '@tabler/icons-react'
import {
  BookmarkCard,
  BookmarkDrawer,
  BookmarkGridSkeleton,
  ConfirmModal,
  EmptyState,
  QueryErrorAlert,
  type BookmarkDraft,
} from '@/components'
import {
  useBookmarks,
  useCategories,
  useDeleteBookmark,
  useSearchBookmarks,
  useVaultUnlock,
} from '@/hooks'
import {
  asStringList,
  buildLibrarySearch,
  categoryPath,
  categorySelectData,
  type SortKey,
} from '@/lib'
import {
  collectTagOptions,
  draftFromBookmark,
  filterAndSortLibraryItems,
  type LibraryListItem,
} from '@/services'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const
const FILTER_PANEL_ID = 'library-filter-panel'

export function LibraryPage(): ReactElement {
  const nav = useNavigate()
  const searchParams = useSearch({ strict: false }) as {
    q?: string
    sort?: SortKey
    category?: string | string[]
    tag?: string | string[]
  }

  const searchRef = useRef<HTMLInputElement>(null)
  const [scroll] = useWindowScroll()

  const filterCats = useMemo(
    () => asStringList(searchParams.category),
    [searchParams.category],
  )
  const filterTags = useMemo(() => asStringList(searchParams.tag), [searchParams.tag])
  const sort: SortKey = searchParams.sort ?? 'updated_desc'
  const urlQ = searchParams.q ?? ''

  const [q, setQ] = useState(urlQ)
  const [debouncedQ] = useDebouncedValue(q, 280)
  const [filterOpen, setFilterOpen] = useState(false)
  const [draftFilterCats, setDraftFilterCats] = useState<string[]>([])
  const [draftFilterTags, setDraftFilterTags] = useState<string[]>([])

  const [drawerOpen, { open: openDrawer, close: closeDrawer }] = useDisclosure(false)
  const [draft, setDraft] = useState<BookmarkDraft | null>(null)
  const [unlockOpen, setUnlockOpen] = useState(false)
  const [vaultPass, setVaultPass] = useState('')
  const [deleteId, setDeleteId] = useState<string | null>(null)

  const cats = useCategories()
  const bookmarks = useBookmarks()
  const searching = debouncedQ.trim().length > 1
  const search = useSearchBookmarks(debouncedQ, searching)
  const deleteBookmark = useDeleteBookmark()
  const vaultUnlock = useVaultUnlock()

  useEffect(() => {
    setQ(urlQ)
  }, [urlQ])

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
      }),
      replace: true,
    })
  }, [debouncedQ]) // eslint-disable-line react-hooks/exhaustive-deps -- sync debounce → URL only

  function setLibrarySearch(patch: {
    q?: string
    sort?: SortKey
    category?: string[]
    tag?: string[]
  }): void {
    void nav({
      to: '/library',
      search: buildLibrarySearch({
        q: patch.q !== undefined ? patch.q : q,
        sort: patch.sort ?? sort,
        category: patch.category ?? filterCats,
        tag: patch.tag ?? filterTags,
      }),
    })
  }

  useEffect(() => {
    if (search.data?.vault_prompt) setUnlockOpen(true)
  }, [search.data?.vault_prompt])

  useHotkeys([
    [
      '/',
      (e) => {
        e.preventDefault()
        searchRef.current?.focus()
      },
    ],
    [
      'mod+K',
      (e) => {
        e.preventDefault()
        searchRef.current?.focus()
      },
    ],
    [
      'n',
      (e) => {
        const t = e.target as HTMLElement | null
        if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable) return
        e.preventDefault()
        openCreate()
      },
    ],
  ])

  const allTags = useMemo(() => collectTagOptions(bookmarks.data || []), [bookmarks.data])

  const catLabel = useMemo(() => {
    const list = cats.data || []
    const byId = new Map(list.map((c) => [c.id, c]))
    const map = new Map<string, string>()
    for (const c of list) map.set(c.id, categoryPath(c, byId))
    return map
  }, [cats.data])

  const list = useMemo((): LibraryListItem[] => {
    const items: LibraryListItem[] = searching
      ? search.data?.hits || []
      : bookmarks.data || []
    return filterAndSortLibraryItems({
      items,
      filterCats,
      filterTags,
      sort,
    })
  }, [bookmarks.data, search.data, searching, filterCats, filterTags, sort])

  function toggleFilter(): void {
    setDraftFilterCats(filterCats)
    setDraftFilterTags(filterTags)
    setFilterOpen((o) => !o)
  }

  function applyFilter(): void {
    setLibrarySearch({ category: draftFilterCats, tag: draftFilterTags })
    setFilterOpen(false)
  }

  function openCreate(): void {
    setDraft(null)
    openDrawer()
  }

  function openEdit(id: string): void {
    const b = (bookmarks.data || []).find((x) => x.id === id)
    if (!b) {
      notifications.show({ color: 'red', message: 'Bookmark not found' })
      return
    }
    setDraft(draftFromBookmark(b))
    openDrawer()
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

  const filterActive = filterCats.length > 0 || filterTags.length > 0
  const loading = bookmarks.isLoading || (searching && search.isFetching && !search.data)
  const emptyLibrary = !loading && !searching && !filterActive && list.length === 0
  const emptyFiltered = !loading && list.length === 0 && (searching || filterActive)

  return (
    <Stack gap="xl">
      <Group justify="space-between" align="flex-end" wrap="wrap" gap="md">
        <Stack gap={4}>
          <Title order={2}>Library</Title>
          {!loading && list.length > 0 ? (
            <Text size="sm" c="dimmed" aria-live="polite">
              {list.length} bookmark{list.length === 1 ? '' : 's'}
              {searching ? ' matched' : ''}
            </Text>
          ) : null}
        </Stack>
        <Tooltip label="New bookmark (N)">
          <Button leftSection={<IconBookmarkPlus {...iconSm} />} onClick={openCreate}>
            Add bookmark
          </Button>
        </Tooltip>
      </Group>

      {cats.isError ? (
        <QueryErrorAlert error={cats.error} onRetry={() => void cats.refetch()} title="Categories failed" />
      ) : null}
      {bookmarks.isError && !searching ? (
        <QueryErrorAlert
          error={bookmarks.error}
          onRetry={() => void bookmarks.refetch()}
          title="Bookmarks failed"
        />
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
                  /
                </Kbd>
              )
            }
            rightSectionPointerEvents={q ? 'all' : 'none'}
            value={q}
            onChange={(e) => setQ(e.currentTarget.value)}
            className={styles.searchInput}
          />
          <Group gap="xs" wrap="nowrap">
            <Tooltip label="Filter by category or tag">
              <Button
                variant={filterOpen || filterActive ? 'light' : 'default'}
                leftSection={<IconFilter {...iconSm} />}
                onClick={toggleFilter}
                aria-expanded={filterOpen}
                aria-controls={FILTER_PANEL_ID}
              >
                Filter
                {filterActive ? ` (${filterCats.length + filterTags.length})` : ''}
              </Button>
            </Tooltip>
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

        <Collapse expanded={filterOpen}>
          <Stack id={FILTER_PANEL_ID} p="lg" gap="md" className={styles.filterPanel}>
            <MultiSelect
              label="Categories"
              data={categorySelectData(cats.data || [])}
              value={draftFilterCats}
              onChange={setDraftFilterCats}
              searchable
              clearable
            />
            <MultiSelect
              label="Tags"
              data={allTags}
              value={draftFilterTags}
              onChange={setDraftFilterTags}
              searchable
              clearable
            />
            <Group justify="flex-end">
              <Button variant="default" onClick={() => setFilterOpen(false)}>
                Cancel
              </Button>
              <Button onClick={applyFilter}>Apply</Button>
            </Group>
          </Stack>
        </Collapse>

        {filterActive ? (
          <Group gap="xs">
            {filterCats.map((id) => (
              <Pill
                key={id}
                withRemoveButton
                onRemove={() =>
                  setLibrarySearch({ category: filterCats.filter((x) => x !== id) })
                }
              >
                {catLabel.get(id) || id}
              </Pill>
            ))}
            {filterTags.map((t) => (
              <Pill
                key={t}
                withRemoveButton
                onRemove={() => setLibrarySearch({ tag: filterTags.filter((x) => x !== t) })}
              >
                #{t}
              </Pill>
            ))}
            <Button
              variant="subtle"
              size="compact-sm"
              onClick={() => setLibrarySearch({ category: [], tag: [] })}
            >
              Clear filters
            </Button>
          </Group>
        ) : null}
      </Stack>

      {loading ? (
        <BookmarkGridSkeleton />
      ) : emptyLibrary ? (
        <EmptyState
          icon={<IconBookmark size={28} stroke={1.5} />}
          title="No bookmarks yet"
          description="Save the first link. Imports live under Settings."
          action={{
            label: 'Add bookmark',
            onClick: openCreate,
            leftSection: <IconBookmarkPlus {...iconSm} />,
          }}
        />
      ) : emptyFiltered ? (
        <EmptyState
          icon={<IconSearch size={28} stroke={1.5} />}
          title="Nothing matched"
          description="Try a different search or clear filters."
          action={
            filterActive || q
              ? {
                  label: 'Clear search & filters',
                  onClick: () => {
                    setQ('')
                    setLibrarySearch({ q: '', category: [], tag: [] })
                  },
                }
              : undefined
          }
        />
      ) : (
        <SimpleGrid cols={{ base: 1, sm: 2, lg: 3, xl: 4 }} spacing="lg">
          {list.map((b) => (
            <BookmarkCard
              key={b.id}
              bookmark={b}
              highlightQuery={searching ? debouncedQ : undefined}
              onEdit={openEdit}
              onDelete={setDeleteId}
            />
          ))}
        </SimpleGrid>
      )}

      <BookmarkDrawer
        opened={drawerOpen}
        onClose={closeDrawer}
        categories={cats.data || []}
        initial={draft}
      />

      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        onConfirm={() => void confirmDelete()}
        title="Delete bookmark?"
        body="This removes the bookmark permanently. You cannot undo this."
        confirmLabel="Delete"
        loading={deleteBookmark.isPending}
      />

      <Modal
        opened={unlockOpen}
        onClose={() => setUnlockOpen(false)}
        title="Unlock vault for search"
      >
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            Settings say vault should be included in search. Unlock to include vault hits.
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
        </Group>
      </Affix>
    </Stack>
  )
}
