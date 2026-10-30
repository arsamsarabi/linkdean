import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
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
import { api, type Bookmark, type SearchHit } from '../api'
import { BookmarkCard } from '../components/BookmarkCard'
import { BookmarkGridSkeleton } from '../components/BookmarkCardSkeleton'
import {
  BookmarkDrawer,
  draftFromBookmark,
  type BookmarkDraft,
} from '../components/BookmarkDrawer'
import { ConfirmModal } from '../components/ConfirmModal'
import { EmptyState } from '../components/EmptyState'
import { categoryPath, categorySelectData } from '../lib/categories'
import { asStringList, buildLibrarySearch, type SortKey } from '../lib/librarySearch'
import { useUi } from '../store'

const iconSm = { size: 16, stroke: 1.5 } as const

type ListItem = Bookmark | (SearchHit & { categories?: Bookmark['categories'] })

export function LibraryPage() {
  const qc = useQueryClient()
  const nav = useNavigate()
  const searchParams = useSearch({ strict: false }) as {
    q?: string
    sort?: SortKey
    category?: string | string[]
    tag?: string | string[]
  }

  const showHidden = useUi((s) => s.showHidden)
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
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
  const [deleting, setDeleting] = useState(false)

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
  }) {
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

  const cats = useQuery({
    queryKey: ['categories', showHidden, vaultUnlocked],
    queryFn: () => api.categories(showHidden, vaultUnlocked),
  })

  const bookmarks = useQuery({
    queryKey: ['bookmarks', vaultUnlocked],
    queryFn: () => api.bookmarks(null, vaultUnlocked),
  })

  const searching = debouncedQ.trim().length > 1
  const search = useQuery({
    queryKey: ['search', debouncedQ],
    queryFn: () => api.search(debouncedQ),
    enabled: searching,
  })

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

  const allTags = useMemo(() => {
    const set = new Set<string>()
    for (const b of bookmarks.data || []) {
      for (const t of b.tags || []) set.add(t)
    }
    return [...set].sort().map((t) => ({ value: t, label: t }))
  }, [bookmarks.data])

  const catLabel = useMemo(() => {
    const list = cats.data || []
    const byId = new Map(list.map((c) => [c.id, c]))
    const map = new Map<string, string>()
    for (const c of list) map.set(c.id, categoryPath(c, byId))
    return map
  }, [cats.data])

  const list = useMemo(() => {
    let items: ListItem[] = searching
      ? search.data?.hits || []
      : bookmarks.data || []

    if (filterCats.length) {
      items = items.filter((b) => {
        const ids = new Set(
          (b.categories || []).map((c) => c.id).concat(b.category_id ? [b.category_id] : []),
        )
        return filterCats.some((id) => ids.has(id))
      })
    }
    if (filterTags.length) {
      items = items.filter((b) => filterTags.every((t) => (b.tags || []).includes(t)))
    }

    const sorted = [...items]
    sorted.sort((a, b) => {
      const at = (a.title || a.url).toLowerCase()
      const bt = (b.title || b.url).toLowerCase()
      const au = 'updated_at' in a && a.updated_at ? a.updated_at : ''
      const bu = 'updated_at' in b && b.updated_at ? b.updated_at : ''
      const ac = 'created_at' in a && a.created_at ? a.created_at : ''
      const bc = 'created_at' in b && b.created_at ? b.created_at : ''
      switch (sort) {
        case 'title_asc':
          return at.localeCompare(bt)
        case 'title_desc':
          return bt.localeCompare(at)
        case 'updated_asc':
          return au.localeCompare(bu)
        case 'created_desc':
          return bc.localeCompare(ac)
        case 'updated_desc':
        default:
          return bu.localeCompare(au)
      }
    })
    return sorted
  }, [bookmarks.data, search.data, searching, filterCats, filterTags, sort])

  function toggleFilter() {
    setDraftFilterCats(filterCats)
    setDraftFilterTags(filterTags)
    setFilterOpen((o) => !o)
  }

  function applyFilter() {
    setLibrarySearch({ category: draftFilterCats, tag: draftFilterTags })
    setFilterOpen(false)
  }

  function openCreate() {
    setDraft(null)
    openDrawer()
  }

  function openEdit(id: string) {
    const b = (bookmarks.data || []).find((x) => x.id === id)
    if (!b) {
      notifications.show({ color: 'red', message: 'Bookmark not found' })
      return
    }
    setDraft(draftFromBookmark(b))
    openDrawer()
  }

  async function unlockForSearch() {
    try {
      await api.vaultUnlock(vaultPass)
      setVaultUnlocked(true)
      setVaultPass('')
      setUnlockOpen(false)
      qc.invalidateQueries({ queryKey: ['vault'] })
      qc.invalidateQueries({ queryKey: ['me'] })
      qc.invalidateQueries({ queryKey: ['categories'] })
      qc.invalidateQueries({ queryKey: ['search'] })
      notifications.show({ message: 'Vault unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function confirmDelete() {
    if (!deleteId) return
    setDeleting(true)
    try {
      await api.deleteBookmark(deleteId)
      qc.invalidateQueries({ queryKey: ['bookmarks'] })
      qc.invalidateQueries({ queryKey: ['search'] })
      notifications.show({ message: 'Bookmark deleted' })
      setDeleteId(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setDeleting(false)
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
            <Text size="sm" c="dimmed">
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
            style={{ flex: 1, minWidth: 200, maxWidth: 440 }}
          />
          <Group gap="xs" wrap="nowrap">
            <Tooltip label="Filter by category or tag">
              <Button
                variant={filterOpen || filterActive ? 'light' : 'default'}
                leftSection={<IconFilter {...iconSm} />}
                onClick={toggleFilter}
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
          <Stack
            p="lg"
            gap="md"
            style={{
              border: '1px solid var(--mantine-color-default-border)',
              borderRadius: 'var(--mantine-radius-md)',
            }}
          >
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
        onConfirm={confirmDelete}
        title="Delete bookmark?"
        body="This removes the bookmark permanently. You cannot undo this."
        confirmLabel="Delete"
        loading={deleting}
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
              onClick={unlockForSearch}
              disabled={vaultPass.length < 8}
            >
              Unlock
            </Button>
          </Group>
        </Stack>
      </Modal>

      <Affix position={{ bottom: 24, right: 24 }}>
        <Group gap="sm">
          <Transition transition="slide-up" mounted={scroll.y > 320}>
            {(styles) => (
              <Tooltip label="Back to top">
                <ActionIcon
                  style={styles}
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
