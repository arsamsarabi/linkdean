import { useMemo, useState } from 'react'
import { notifications } from '@mantine/notifications'
import type { BulkEditMode } from '@/components'
import { useBulkAddCategories, useBulkDeleteBookmarks, useBulkTagBookmarks } from '@/hooks/bookmarks'
import { useSyncDuringRender } from '@/hooks/render'
import { toggleId, writableIds, type LibraryListItem } from '@/services'

type Input = {
  list: LibraryListItem[]
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

/** Multi-select + bulk category/tag/delete actions. */
export function useLibrarySelection({ list }: Input) {
  const [selectMode, setSelectMode] = useState(false)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [bulkMode, setBulkMode] = useState<BulkEditMode | null>(null)
  const [bulkDeleteOpen, setBulkDeleteOpen] = useState(false)

  const bulkCategories = useBulkAddCategories()
  const bulkTag = useBulkTagBookmarks()
  const bulkDelete = useBulkDeleteBookmarks()
  const bulkPending = bulkCategories.isPending || bulkTag.isPending || bulkDelete.isPending

  const selectableIds = useMemo(() => writableIds(list), [list])

  // What leaves the screen (a new search, folder or filter) leaves the selection, so bulk
  // actions and their warnings only ever cover what the user can see.
  useSyncDuringRender([selectableIds], () => {
    setSelectedIds((prev) => {
      const visible = new Set(selectableIds)
      const kept = [...prev].filter((id) => visible.has(id))
      return kept.length === prev.size ? prev : new Set(kept)
    })
  })

  const selectedItems = useMemo(() => list.filter((b) => selectedIds.has(b.id)), [list, selectedIds])
  const selectedCount = selectedIds.size
  const allVisibleSelected = selectableIds.length > 0 && selectableIds.every((id) => selectedIds.has(id))

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
    allVisibleSelected,
    exitSelectMode,
    enterSelectMode,
    toggleSelect,
    applyBulkCategories,
    applyBulkTag,
    confirmBulkDelete,
  }
}
