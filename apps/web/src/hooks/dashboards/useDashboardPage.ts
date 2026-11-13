import { useEffect, useMemo, useState } from 'react'
import { notifications } from '@mantine/notifications'
import { useMe } from '@/hooks/auth'
import { useTagNames } from '@/hooks/bookmarks'
import { useCategories } from '@/hooks/categories'
import { categoryPath } from '@/lib'
import { dashboardToShowInstead, layoutWithPin, mergeDashboardBookmarks, sectionFillsItself } from '@/services'
import { useDashboard, usePutDashboard } from './useDashboard'
import { useDashboardBoard } from './useDashboardBoard'
import { useDashboardEdit } from './useDashboardEdit'
import { useDashboardShares } from './useDashboardShares'

/** Compose dashboard board + edit-draft hooks for DashboardPage. */
// Known issue: An edit draft is not tied to its dashboard, so after losing access to one it can be
// saved onto another. Fix: bind the draft to its dashboard id.
export function useDashboardPage() {
  const me = useMe()
  const board = useDashboardBoard()
  const dashboard = useDashboard(board.activeId)
  const save = usePutDashboard()
  const edit = useDashboardEdit({
    activeId: board.activeId,
    dashboard: dashboard.data,
    save,
  })
  const { activeId, list, showDashboard } = board
  const loadFailed = dashboard.isError

  // Access may have been revoked while this board was open.
  useEffect(() => {
    if (!loadFailed) return
    void list.refetch().then(({ data }) => {
      const next = dashboardToShowInstead(data ?? [], activeId)
      if (next) showDashboard(next)
    })
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- refetch and setter are stable
  }, [loadFailed, activeId])

  const viewerUserId = me.data?.user.id
  const viewerName = me.data?.user.display_name?.trim().split(/\s+/)[0] || null
  const [filter, setFilter] = useState('')
  const view = dashboard.data
  const access = board.accessFlags(view)

  const bookmarks = useMemo(
    () => mergeDashboardBookmarks(board.libraryBookmarks ?? [], view),
    [board.libraryBookmarks, view]
  )
  const bookmarkOptions = useMemo(() => bookmarks.map((b) => ({ value: b.id, label: b.title || b.url })), [bookmarks])
  const bookmarkById = useMemo(() => new Map(bookmarks.map((b) => [b.id, b])), [bookmarks])

  // Sources for sections that fill themselves: the owner's own folders outside vaults, and tags.
  const categories = useCategories(true, false)
  const tags = useTagNames()
  const folders = useMemo(() => {
    const own = (categories.data ?? []).filter((c) => !c.vault_id && c.access === 'owner')
    const byId = new Map(own.map((c) => [c.id, c]))
    return own
      .map((c) => ({ value: c.id, label: categoryPath(c, byId) }))
      .sort((a, b) => a.label.localeCompare(b.label))
  }, [categories.data])
  const folderLabel = useMemo(() => new Map(folders.map((f) => [f.value, f.label])), [folders])
  const folderName = (id: string): string | undefined => folderLabel.get(id)

  // A shared dashboard, or one with an invite waiting, takes pinned sections only.
  const shares = useDashboardShares(access.isOwner && edit.editing ? (board.activeId ?? null) : null)
  const hasShares = !!view?.shared_out || (shares.data?.pending.length ?? 0) > 0
  const canAddSelfFilling = access.isOwner && !hasShares
  const selfFillingSections = (view?.sections ?? []).filter(sectionFillsItself).map((s) => s.title)

  /** Pins a bookmark to a section straight from the dashboard. True once saved. */
  async function quickPin(sectionId: string, bookmarkId: string): Promise<boolean> {
    if (!view || !board.activeId) return false
    try {
      await save.mutateAsync({ id: board.activeId, body: layoutWithPin(view, sectionId, bookmarkId) })
      notifications.show({ message: 'Pinned' })
      return true
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
      return false
    }
  }

  return {
    list: board.list,
    viewerName,
    filter,
    setFilter,
    quickPin,
    dashboard,
    activeId: board.activeId,
    viewerUserId,
    editDraft: edit.editDraft,
    deleteOpen: board.deleteOpen,
    setDeleteOpen: board.setDeleteOpen,
    shareOpen: board.shareOpen,
    setShareOpen: board.setShareOpen,
    socialOpen: board.socialOpen,
    setSocialOpen: board.setSocialOpen,
    bookmarkOptions,
    bookmarkById,
    folderOptions: folders,
    folderName,
    tagOptions: tags.data ?? [],
    canAddSelfFilling,
    selfFillingSections,
    switcherData: board.switcherData,
    save,
    create: board.create,
    patch: board.patch,
    remove: board.remove,
    view,
    ...access,
    startEdit: edit.startEdit,
    cancelEdit: edit.cancelEdit,
    addSection: edit.addSection,
    updateSection: edit.updateSection,
    applyLayout: edit.applyLayout,
    movePin: edit.movePin,
    selectDashboard: (id: string | null) => board.selectDashboard(id, edit.editing),
    onSave: edit.onSave,
    onCreate: () => board.onCreate(edit.editing),
    onPin: board.onPin,
    onDelete: () => board.onDelete(edit.clearDraft),
    updateSectionTitle: edit.updateSectionTitle,
    updateSectionBookmarks: edit.updateSectionBookmarks,
    removeSection: edit.removeSection,
    setDraftTitle: edit.setDraftTitle,
  }
}
