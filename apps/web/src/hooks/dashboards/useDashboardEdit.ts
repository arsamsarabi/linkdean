import { useState } from 'react'
import { notifications } from '@mantine/notifications'
import type { Dashboard, DashboardSectionKind, PutDashboardLayout } from '@/schemas'
import {
  dashboardDraftError,
  draftFromDashboard,
  emptyDashboardDraft,
  moveItem,
  newDraftSection,
  toPutDashboardLayout,
  type DashboardDraft,
  type DashboardDraftSection,
} from '@/services'

function buildEditDraft(data: Dashboard | undefined): DashboardDraft {
  if (!data) return emptyDashboardDraft()
  const next = draftFromDashboard(data)
  if (next.sections.length > 0) return next
  return { ...next, sections: [newDraftSection('Favorites')] }
}

type Input = {
  activeId: string | undefined
  dashboard: Dashboard | undefined
  save: {
    mutateAsync: (vars: { id: string; body: PutDashboardLayout }) => Promise<Dashboard>
  }
}

/** Edit-draft state + section mutators for the dashboard layout editor. */
export function useDashboardEdit({ activeId, dashboard, save }: Input) {
  const [editDraft, setEditDraft] = useState<DashboardDraft | null>(null)
  const editing = editDraft !== null

  function startEdit(): void {
    try {
      setEditDraft(buildEditDraft(dashboard))
    } catch (e) {
      notifications.show({ color: 'red', message: `Could not start edit: ${String(e)}` })
    }
  }

  function cancelEdit(): void {
    setEditDraft(null)
  }

  /** Adds a section at the end and returns its key, so its settings can open straight away. */
  function addSection(kind: DashboardSectionKind = 'pinned'): string {
    const section = newDraftSection(undefined, kind)
    setEditDraft((prev) => {
      if (!prev) return prev
      return { ...prev, sections: [...prev.sections, section] }
    })
    return section.key
  }

  /** Drag and drop: moves a section from one place on the board to another. */
  function moveSectionTo(from: number, to: number): void {
    setEditDraft((prev) => (prev ? { ...prev, sections: moveItem(prev.sections, from, to) } : prev))
  }

  /** Drag and drop: moves a pinned bookmark within its section. */
  function movePin(idx: number, from: number, to: number): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      const next = [...prev.sections]
      const section = next[idx]
      if (!section) return prev
      next[idx] = { ...section, bookmark_ids: moveItem(section.bookmark_ids, from, to) }
      return { ...prev, sections: next }
    })
  }

  /** Changes a section's display mode, number of bookmarks, source or counting window. */
  function updateSection(
    idx: number,
    patch: Partial<Omit<DashboardDraftSection, 'key' | 'kind' | 'bookmark_ids'>>
  ): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      const next = [...prev.sections]
      const section = next[idx]
      if (!section) return prev
      next[idx] = { ...section, ...patch }
      return { ...prev, sections: next }
    })
  }

  async function onSave(): Promise<void> {
    if (!editDraft || !activeId) return
    const invalid = dashboardDraftError(editDraft)
    if (invalid) {
      notifications.show({ color: 'red', message: invalid })
      return
    }
    try {
      await save.mutateAsync({ id: activeId, body: toPutDashboardLayout(editDraft) })
      notifications.show({ message: 'Dashboard saved' })
      setEditDraft(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  function updateSectionTitle(idx: number, title: string): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      const next = [...prev.sections]
      const section = next[idx]
      if (!section) return prev
      next[idx] = { ...section, title }
      return { ...prev, sections: next }
    })
  }

  function updateSectionBookmarks(idx: number, bookmark_ids: string[]): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      const next = [...prev.sections]
      const section = next[idx]
      if (!section) return prev
      next[idx] = { ...section, bookmark_ids }
      return { ...prev, sections: next }
    })
  }

  function removeSection(idx: number): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      return { ...prev, sections: prev.sections.filter((_, i) => i !== idx) }
    })
  }

  function setDraftTitle(title: string): void {
    setEditDraft((prev) => (prev ? { ...prev, title } : prev))
  }

  function clearDraft(): void {
    setEditDraft(null)
  }

  return {
    editDraft,
    editing,
    startEdit,
    cancelEdit,
    addSection,
    moveSectionTo,
    movePin,
    updateSection,
    onSave,
    updateSectionTitle,
    updateSectionBookmarks,
    removeSection,
    setDraftTitle,
    clearDraft,
  }
}
