import { useState } from 'react'
import { notifications } from '@mantine/notifications'
import type { Dashboard, PutDashboardLayout } from '@/schemas'
import {
  dashboardDraftError,
  emptyDashboardDraft,
  moveSection,
  newDraftSection,
  toPutDashboardLayout,
  type DashboardDraft,
} from '@/services'

function draftFromDashboard(d: Dashboard): DashboardDraft {
  return {
    title: d.title,
    sections: d.sections.map((s) => ({
      key: s.id,
      title: s.title,
      bookmark_ids: s.items.map((i) => i.bookmark.id),
    })),
  }
}

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

  function addSection(): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      return { ...prev, sections: [...prev.sections, newDraftSection()] }
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

  function moveSectionBy(idx: number, delta: -1 | 1): void {
    setEditDraft((prev) => {
      if (!prev) return prev
      return { ...prev, sections: moveSection(prev.sections, idx, delta) }
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
    onSave,
    updateSectionTitle,
    updateSectionBookmarks,
    moveSectionBy,
    removeSection,
    setDraftTitle,
    clearDraft,
  }
}
