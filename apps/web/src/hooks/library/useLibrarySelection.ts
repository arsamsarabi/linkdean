import { useMemo, useRef, useState } from 'react'
import { notifications } from '@mantine/notifications'
import { api } from '@/api'
import type { BulkEditMode } from '@/components'
import { useBulkAddCategories, useBulkDeleteBookmarks, useBulkTagBookmarks } from '@/hooks/bookmarks'
import { useSyncDuringRender } from '@/hooks/render'
import { toggleId, writableIds, type LibraryListItem } from '@/services'
import type { LibraryPageFilter } from './useLibraryBookmarksInfinite'

type Input = {
  list: LibraryListItem[]
  /** Server filter that `/bookmarks/ids` can match exactly; null when the screen is client-filtered. */
  matchFilter?: LibraryPageFilter | null
}

// Known issue: Bulk failures carry no detail, and the selection clears even on partial failure.
// Fix: keep the failed ids selected, with the reason.
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

/**
 * Multi-select + bulk category/tag/delete actions. Select all asks the server for every id the
 * current filter matches (`matched`), so it reaches rows beyond the loaded pages.
 */
export function useLibrarySelection({ list, matchFilter = null }: Input) {
  const [selectMode, setSelectMode] = useState(false)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [matched, setMatched] = useState<{ key: string; count: number } | null>(null)
  const [selectingAll, setSelectingAll] = useState(false)
  const matchKey = JSON.stringify(matchFilter)
  const [bulkMode, setBulkMode] = useState<BulkEditMode | null>(null)
  const [bulkDeleteOpen, setBulkDeleteOpen] = useState(false)

  const bulkCategories = useBulkAddCategories()
  const bulkTag = useBulkTagBookmarks()
  const bulkDelete = useBulkDeleteBookmarks()
  const bulkPending = bulkCategories.isPending || bulkTag.isPending || bulkDelete.isPending

  const selectableIds = useMemo(() => writableIds(list), [list])

  // What leaves the screen (a new search, folder or filter) leaves the selection, so bulk
  // actions and their warnings only ever cover what the user can see.
  useSyncDuringRender([selectableIds, matchKey, matched], () => {
    // A match-all selection outlives the loaded pages, but not its filter.
    if (matched?.key === matchKey) return
    if (matched) setMatched(null)
    setSelectedIds((prev) => {
      const visible = new Set(selectableIds)
      const kept = [...prev].filter((id) => visible.has(id))
      return kept.length === prev.size ? prev : new Set(kept)
    })
  })

  const selectedItems = useMemo(() => list.filter((b) => selectedIds.has(b.id)), [list, selectedIds])
  const selectedCount = selectedIds.size
  const allSelected = matched
    ? selectedCount > 0 && selectedCount === matched.count
    : selectableIds.length > 0 && selectableIds.every((id) => selectedIds.has(id))

  function exitSelectMode(): void {
    anchorRef.current = null
    setSelectMode(false)
    setSelectedIds(new Set())
    setMatched(null)
    setBulkMode(null)
    setBulkDeleteOpen(false)
  }

  function enterSelectMode(): void {
    setSelectMode(true)
    setSelectedIds(new Set())
    setMatched(null)
  }

  /** Select (or, when everything is already selected, clear) every bookmark the filter matches. */
  async function toggleSelectAll(): Promise<void> {
    if (allSelected) {
      setSelectedIds(new Set())
      setMatched(null)
      return
    }
    if (!matchFilter) {
      setSelectedIds(new Set(selectableIds))
      return
    }
    setSelectingAll(true)
    try {
      const readOnly = new Set(list.filter((b) => b.can_write === false).map((b) => b.id))
      // Ids carry no ACL, so only loaded rows can be screened; the server refuses the rest on mutate.
      const ids = (await api.bookmarkIds(matchFilter)).filter((id) => !readOnly.has(id))
      setSelectedIds(new Set(ids))
      setMatched({ key: matchKey, count: ids.length })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setSelectingAll(false)
    }
  }

  const anchorRef = useRef<string | null>(null)

  /**
   * Ticking a box starts selecting, so no separate Select step is needed. With `range` (shift
   * held), every bookmark between the last ticked one and this one is selected.
   */
  function toggleSelect(id: string, range = false): void {
    setSelectMode(true)
    const anchor = anchorRef.current
    if (range && anchor && anchor !== id) {
      const from = selectableIds.indexOf(anchor)
      const to = selectableIds.indexOf(id)
      if (from >= 0 && to >= 0) {
        const [lo, hi] = from < to ? [from, to] : [to, from]
        setSelectedIds((prev) => new Set([...prev, ...selectableIds.slice(lo, hi + 1)]))
        anchorRef.current = id
        return
      }
    }
    anchorRef.current = id
    setSelectedIds((prev) => toggleId(prev, id))
  }

  async function applyBulkCategories(categoryIds: string[]): Promise<void> {
    const ids = [...selectedIds]
    if (ids.length === 0) return
    const result = await bulkCategories.mutateAsync({ ids, category_ids: categoryIds })
    notifyBulk('Categories added', result)
    setBulkMode(null)
    exitSelectMode()
  }

  async function applyBulkTag(tags: string[], tagMode: 'add' | 'replace'): Promise<void> {
    const ids = [...selectedIds]
    if (ids.length === 0) return
    const result = await bulkTag.mutateAsync({ ids, tags, mode: tagMode })
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

  return {
    selectMode,
    selectedIds,
    setSelectedIds,
    bulkMode,
    setBulkMode,
    bulkDeleteOpen,
    setBulkDeleteOpen,
    bulkDelete,
    bulkPending,
    selectableIds,
    selectedItems,
    selectedCount,
    allSelected,
    selectingAll,
    toggleSelectAll,
    exitSelectMode,
    enterSelectMode,
    toggleSelect,
    applyBulkCategories,
    applyBulkTag,
    confirmBulkDelete,
  }
}
