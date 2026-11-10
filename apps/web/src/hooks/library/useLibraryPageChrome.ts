import { useEffect, useRef } from 'react'
import { useHotkeys } from '@mantine/hooks'
import { useNavigate } from '@tanstack/react-router'
import { useScrollY } from '@/hooks/render'
import { buildLibrarySearch, type SortKey } from '@/lib'
import type { ExplorerFolderId, FlagFilter, SinceFilter } from '@/services'
import { isModalDialogOpen } from '../render'
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
  const scrollY = useScrollY()

  useHotkeys([
    [
      // `/` searches this page's bookmarks; mod+K is the command palette.
      '/',
      (e) => {
        const t = e.target as HTMLElement | null
        if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable) return
        e.preventDefault()
        searchRef.current?.focus()
      },
    ],
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
      // Esc in a dialog or drawer closes that, and leaves the library selection alone.
      'Escape',
      () => {
        if (selectMode && !isModalDialogOpen()) exitSelectMode()
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
    scrollY,
  }
}
