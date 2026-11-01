import { useEffect, useMemo, useState } from 'react'
import { useDebouncedValue } from '@mantine/hooks'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { asStringList, buildLibrarySearch, type FlagKey, type SortKey } from '@/lib'
import { FOLDER_ALL, normalizeFolderId, type FlagFilter, type SinceFilter } from '@/services'
import type { LibrarySearchParams, LibrarySearchPatch } from './types'

/** URL-backed library search/filter state + draft filter panel. */
export function useLibraryUrlState() {
  const nav = useNavigate()
  const searchParams = useSearch({ strict: false }) as LibrarySearchParams

  const filterCats = useMemo(() => asStringList(searchParams.category), [searchParams.category])
  const filterTags = useMemo(() => asStringList(searchParams.tag), [searchParams.tag])
  const filterFlags = useMemo(
    () =>
      asStringList(searchParams.flag).filter((f): f is FlagFilter =>
        ['unread', 'favorite', 'pinned'].includes(f)
      ),
    [searchParams.flag]
  )
  const filterSince: SinceFilter | null = searchParams.since ?? null
  const vaultOnly = searchParams.vault === '1'
  const sort: SortKey = searchParams.sort ?? 'updated_desc'
  const urlQ = searchParams.q ?? ''
  const folderId = normalizeFolderId(searchParams.folder)

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

  function setLibrarySearch(patch: LibrarySearchPatch): void {
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

  const panelFilterActive =
    filterCats.length > 0 || filterTags.length > 0 || !!filterSince || vaultOnly
  const panelFilterCount =
    filterCats.length + filterTags.length + (filterSince ? 1 : 0) + (vaultOnly ? 1 : 0)
  const filterActive = panelFilterActive || filterFlags.length > 0

  return {
    searchParams,
    filterCats,
    filterTags,
    filterFlags,
    filterSince,
    vaultOnly,
    sort,
    urlQ,
    folderId,
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
    setLibrarySearch,
    toggleFlagFilter,
    toggleFilter,
    applyFilter,
    panelFilterActive,
    panelFilterCount,
    filterActive,
  }
}
