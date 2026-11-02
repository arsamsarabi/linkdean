import { useEffect, useRef } from 'react'
import { useHotkeys, useWindowScroll } from '@mantine/hooks'
import { useNavigate } from '@tanstack/react-router'
import { buildLibrarySearch, type SortKey } from '@/lib'
import type { ExplorerFolderId, FlagFilter, SinceFilter } from '@/services'
import type { LibraryPath, LibrarySearchParams } from './types'

type Input = {
  basePath: LibraryPath
  vaultId: string | null
  canCreate: boolean
  selectMode: boolean
  openCreate: () => void
  openCreateWithUrl: (url: string) => void
  exitSelectMode: () => void
  searchParams: LibrarySearchParams
  urlQ: string
  sort: SortKey
  filterCats: string[]
  filterTags: string[]
  filterFlags: FlagFilter[]
  filterSince: SinceFilter | null
  vaultOnly: boolean
  folderId: ExplorerFolderId | undefined
}

/** Hotkeys, scroll, and one-shot palette URL flags for Library page. */
export function useLibraryPageChrome({
  basePath,
  vaultId,
  canCreate,
  selectMode,
  openCreate,
  openCreateWithUrl,
  exitSelectMode,
  searchParams,
  urlQ,
  sort,
  filterCats,
  filterTags,
  filterFlags,
  filterSince,
  vaultOnly,
  folderId,
}: Input) {
  const nav = useNavigate()
  const searchRef = useRef<HTMLInputElement>(null)
  const [scroll] = useWindowScroll()

  useHotkeys([
    [
      'n',
      (e) => {
        const t = e.target as HTMLElement | null
        if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable) return
        if (!canCreate) return
        e.preventDefault()
        openCreate()
      },
    ],
    [
      // Known issue: Esc in any dialog or drawer also clears the library selection. Fix: skip while
      // a dialog is open, as `useDrawerClosesOnEscape` in hooks/render does.
      'Escape',
      () => {
        if (selectMode) exitSelectMode()
      },
    ],
  ])

  /** Consume one-shot flags (`create`, `addUrl` from paste to add, `focus`) then strip from URL. */
  useEffect(() => {
    if (!searchParams.create && !searchParams.addUrl && searchParams.focus !== 'search') return
    if (searchParams.addUrl) openCreateWithUrl(searchParams.addUrl)
    else if (searchParams.create) openCreate()
    if (searchParams.focus === 'search') {
      window.requestAnimationFrame(() => searchRef.current?.focus())
    }
    void nav({
      to: basePath,
      search: buildLibrarySearch({
        q: urlQ,
        sort,
        category: filterCats,
        tag: filterTags,
        flag: filterFlags,
        since: filterSince,
        vault: vaultOnly,
        vaultId,
        folder: folderId,
      }),
      replace: true,
    })
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- one-shot URL flags only
  }, [searchParams.create, searchParams.addUrl, searchParams.focus])

  return {
    searchRef,
    scroll,
  }
}
