import { useEffect, useRef } from 'react'
import { useDisclosure, useHotkeys, useWindowScroll } from '@mantine/hooks'
import { useNavigate } from '@tanstack/react-router'
import { buildLibrarySearch, type SortKey } from '@/lib'
import type { ExplorerFolderId, FlagFilter, SinceFilter } from '@/services'
import type { LibrarySearchParams } from './types'

type Input = {
  canCreate: boolean
  selectMode: boolean
  openCreate: () => void
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

/** Hotkeys, scroll, dupes modal, and one-shot palette URL flags for Library page. */
export function useLibraryPageChrome({
  canCreate,
  selectMode,
  openCreate,
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
  const [dupesOpen, { open: openDupes, close: closeDupes }] = useDisclosure(false)

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

  return {
    searchRef,
    scroll,
    dupesOpen,
    openDupes,
    closeDupes,
  }
}
