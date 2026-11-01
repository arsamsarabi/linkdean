import { useEffect, useMemo, useState } from 'react'
import { notifications } from '@mantine/notifications'
import { useBookmarks } from '@/hooks/bookmarks'
import type { Dashboard } from '@/schemas'
import {
  useCreateDashboard,
  useDashboards,
  useDefaultDashboard,
  useDeleteDashboard,
  usePatchDashboard,
} from './useDashboard'

/** Active board selection, bookmark option maps, create/pin/delete, modal open flags. */
export function useDashboardBoard() {
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

  const bookmarks = useBookmarks()
  const create = useCreateDashboard()
  const patch = usePatchDashboard()
  const remove = useDeleteDashboard()
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [shareOpen, setShareOpen] = useState(false)
  const [socialOpen, setSocialOpen] = useState(false)

  const switcherData = useMemo(() => {
    return (list.data || []).map((d) => {
      const shared = d.access === 'read' || d.access === 'write'
      let label = d.title
      if (d.is_default) label = `${label} (default)`
      else if (shared) label = `${label} (shared)`
      return { value: d.id, label }
    })
  }, [list.data])

  function selectDashboard(id: string | null, editing: boolean): void {
    if (!id || id === activeId) return
    if (editing) {
      notifications.show({ color: 'yellow', message: 'Save or cancel edit before switching' })
      return
    }
    setActiveId(id)
  }

  async function onCreate(editing: boolean): Promise<void> {
    if (editing) {
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

  async function onDelete(clearDraft: () => void): Promise<void> {
    if (!activeId) return
    try {
      const remaining = await remove.mutateAsync(activeId)
      const next = remaining.find((d) => d.is_default)?.id || remaining[0]?.id
      setActiveId(next)
      setDeleteOpen(false)
      clearDraft()
      notifications.show({ message: 'Dashboard deleted' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  function accessFlags(view: Dashboard | undefined) {
    const empty = !!view && view.sections.length === 0
    const isPinned = !!view?.is_default
    const isOwner = view?.access === 'owner'
    const canEditLayout = view?.access === 'owner' || view?.access === 'write'
    const inviteeAccess = view?.access === 'read' || view?.access === 'write' ? view.access : null
    const socialEnabled = !!inviteeAccess || !!view?.shared_out
    return { empty, isPinned, isOwner, canEditLayout, inviteeAccess, socialEnabled }
  }

  return {
    list,
    activeId,
    libraryBookmarks: bookmarks.data,
    showDashboard: setActiveId,
    switcherData,
    create,
    patch,
    remove,
    deleteOpen,
    setDeleteOpen,
    shareOpen,
    setShareOpen,
    socialOpen,
    setSocialOpen,
    selectDashboard,
    onCreate,
    onPin,
    onDelete,
    accessFlags,
  }
}
