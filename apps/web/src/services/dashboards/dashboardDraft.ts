import { newClientId } from '@/lib/id'
import type { Bookmark, Dashboard, DashboardSummary } from '@/schemas'

export type DashboardDraftSection = {
  key: string
  title: string
  bookmark_ids: string[]
}

export type DashboardDraft = {
  title: string
  sections: DashboardDraftSection[]
}

export function emptyDashboardDraft(): DashboardDraft {
  return {
    title: 'Home',
    sections: [{ key: newClientId(), title: 'Favourites', bookmark_ids: [] }],
  }
}

export function newDraftSection(title = 'New section'): DashboardDraftSection {
  return { key: newClientId(), title, bookmark_ids: [] }
}

export function moveSection<T>(sections: readonly T[], index: number, delta: number): T[] {
  const next = [...sections]
  const target = index + delta
  if (index < 0 || index >= next.length || target < 0 || target >= next.length) {
    return next
  }
  const tmp = next[index]!
  next[index] = next[target]!
  next[target] = tmp
  return next
}

/** Why the draft cannot be saved, or null. A blank section title would lose its pins. */
export function dashboardDraftError(draft: DashboardDraft): string | null {
  return draft.sections.some((s) => !s.title.trim()) ? 'Every section needs a title.' : null
}

export function toPutDashboardLayout(draft: DashboardDraft): {
  title: string
  sections: { title: string; bookmark_ids: string[] }[]
} {
  return {
    title: draft.title.trim() || 'Home',
    sections: draft.sections.map((s) => ({
      title: s.title.trim(),
      bookmark_ids: s.bookmark_ids,
    })),
  }
}

/**
 * The editor's library plus everything pinned on the dashboard. Pins on a shared
 * dashboard usually belong to its owner, so they are not in the editor's library.
 */
export function mergeDashboardBookmarks(library: Bookmark[], dashboard: Dashboard | undefined): Bookmark[] {
  const byId = new Map(library.map((b) => [b.id, b]))
  for (const section of dashboard?.sections ?? []) {
    for (const { bookmark } of section.items) {
      if (!byId.has(bookmark.id)) byId.set(bookmark.id, bookmark)
    }
  }
  return [...byId.values()]
}

/** Where to go when the open dashboard failed to load and has left the user's list. */
export function dashboardToShowInstead(
  list: Pick<DashboardSummary, 'id' | 'is_default'>[],
  activeId: string | undefined
): string | undefined {
  if (!activeId || list.some((d) => d.id === activeId)) return undefined
  return list.find((d) => d.is_default)?.id ?? list[0]?.id
}
