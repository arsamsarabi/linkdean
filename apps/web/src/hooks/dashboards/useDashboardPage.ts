import { useEffect, useMemo, useState } from 'react'
import { notifications } from '@mantine/notifications'
import { useMe } from '@/hooks/auth'
import { useBookmarks } from '@/hooks/bookmarks'
import {
  useCreateDashboard,
  useDashboard,
  useDashboards,
  useDefaultDashboard,
  useDeleteDashboard,
  usePatchDashboard,
  usePutDashboard,
} from './useDashboard'
import { useDashboardEdit } from './useDashboardEdit'

/** Compose dashboard list / active board / edit-draft hooks for DashboardPage. */
export function useDashboardPage() {
  const me = useMe()
  const list = useDashboards()
  const defaultDash = useDefaultDashboard()
  const [activeId, setActiveId] = useState<string | undefined>(undefined)
  const pinnedId = list.data?.find((d) => d.is_default)?.id || defaultDash.data?.id

  useEffect(() => {
    if (!activeId && pinnedId) {
      // oxlint-disable-next-line react/set-state-in-effect -- pick pin once list/default known
      setActiveId(pinnedId)
    }
  }, [activeId, pinnedId])

  const dashboard = useDashboard(activeId)
  const bookmarks = useBookmarks()
  const save = usePutDashboard()
  const create = useCreateDashboard()
  const patch = usePatchDashboard()
  const remove = useDeleteDashboard()
  const viewerUserId = me.data?.user.id
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [shareOpen, setShareOpen] = useState(false)
  const [socialOpen, setSocialOpen] = useState(false)

  const edit = useDashboardEdit({
    activeId,
    dashboard: dashboard.data,
    save,
  })

  const bookmarkOptions = useMemo(() => {
    return (bookmarks.data || []).map((b) => ({
      value: b.id,
      label: b.title || b.url,
    }))
  }, [bookmarks.data])

  const bookmarkById = useMemo(() => {
    return new Map((bookmarks.data || []).map((b) => [b.id, b]))
  }, [bookmarks.data])

  const switcherData = useMemo(() => {
    return (list.data || []).map((d) => {
      const shared = d.access === 'read' || d.access === 'write'
      let label = d.title
      if (d.is_default) label = `${label} (default)`
      else if (shared) label = `${label} (shared)`
      return { value: d.id, label }
    })
  }, [list.data])

  function selectDashboard(id: string | null): void {
    if (!id || id === activeId) return
    if (edit.editing) {
      notifications.show({ color: 'yellow', message: 'Save or cancel edit before switching' })
      return
    }
    setActiveId(id)
  }

  async function onCreate(): Promise<void> {
    if (edit.editing) {
      notifications.show({ color: 'yellow', message: 'Save or cancel edit first' })
      return
    }
    try {
      const created = await create.mutateAsync({ title: 'New dashboard' })
      setActiveId(created.id)
      notifications.show({ message: 'Dashboard created' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onPin(): Promise<void> {
    if (!activeId) return
    try {
      await patch.mutateAsync({ id: activeId, body: { is_default: true } })
      notifications.show({ message: 'Default dashboard updated' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onDelete(): Promise<void> {
    if (!activeId) return
    try {
      const remaining = await remove.mutateAsync(activeId)
      const next = remaining.find((d) => d.is_default)?.id || remaining[0]?.id
      setActiveId(next)
      setDeleteOpen(false)
      edit.clearDraft()
      notifications.show({ message: 'Dashboard deleted' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const view = dashboard.data
  const empty = !!view && view.sections.length === 0
  const isPinned = !!view?.is_default
  const isOwner = view?.access === 'owner'
  const canEditLayout = view?.access === 'owner' || view?.access === 'write'
  const inviteeAccess =
    view?.access === 'read' || view?.access === 'write' ? view.access : null
  const socialEnabled = !!inviteeAccess || !!view?.shared_out

  return {
    list,
    dashboard,
    activeId,
    viewerUserId,
    editDraft: edit.editDraft,
    deleteOpen,
    setDeleteOpen,
    shareOpen,
    setShareOpen,
    socialOpen,
    setSocialOpen,
    bookmarkOptions,
    bookmarkById,
    switcherData,
    save,
    create,
    patch,
    remove,
    view,
    empty,
    isPinned,
    isOwner,
    canEditLayout,
    inviteeAccess,
    socialEnabled,
    startEdit: edit.startEdit,
    cancelEdit: edit.cancelEdit,
    addSection: edit.addSection,
    selectDashboard,
    onSave: edit.onSave,
    onCreate,
    onPin,
    onDelete,
    updateSectionTitle: edit.updateSectionTitle,
    updateSectionBookmarks: edit.updateSectionBookmarks,
    moveSectionBy: edit.moveSectionBy,
    removeSection: edit.removeSection,
    setDraftTitle: edit.setDraftTitle,
  }
}
