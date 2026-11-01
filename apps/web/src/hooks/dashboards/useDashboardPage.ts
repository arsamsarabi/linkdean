import { useMemo } from 'react'
import { useMe } from '@/hooks/auth'
import { mergeDashboardBookmarks } from '@/services'
import { useDashboard, usePutDashboard } from './useDashboard'
import { useDashboardBoard } from './useDashboardBoard'
import { useDashboardEdit } from './useDashboardEdit'

/** Compose dashboard board + edit-draft hooks for DashboardPage. */
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
  const viewerUserId = me.data?.user.id
  const view = dashboard.data
  const access = board.accessFlags(view)

  const bookmarks = useMemo(
    () => mergeDashboardBookmarks(board.libraryBookmarks ?? [], view),
    [board.libraryBookmarks, view]
  )
  const bookmarkOptions = useMemo(() => bookmarks.map((b) => ({ value: b.id, label: b.title || b.url })), [bookmarks])
  const bookmarkById = useMemo(() => new Map(bookmarks.map((b) => [b.id, b])), [bookmarks])

  return {
    list: board.list,
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
    selectDashboard: (id: string | null) => board.selectDashboard(id, edit.editing),
    onSave: edit.onSave,
    onCreate: () => board.onCreate(edit.editing),
    onPin: board.onPin,
    onDelete: () => board.onDelete(edit.clearDraft),
    updateSectionTitle: edit.updateSectionTitle,
    updateSectionBookmarks: edit.updateSectionBookmarks,
    moveSectionBy: edit.moveSectionBy,
    removeSection: edit.removeSection,
    setDraftTitle: edit.setDraftTitle,
  }
}
