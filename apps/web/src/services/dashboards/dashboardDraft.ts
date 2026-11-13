import { newClientId } from '@/lib/id'
import {
  DASHBOARD_SECTION_DEFAULT_ITEMS,
  type Bookmark,
  type Dashboard,
  type DashboardSectionKind,
  type DashboardSectionSettings,
  type DashboardSummary,
} from '@/schemas'

export type DashboardDraftSection = DashboardSectionSettings & {
  key: string
  title: string
  bookmark_ids: string[]
}

/** Default title for a new section of each kind. */
export const SECTION_KIND_TITLES: Record<DashboardSectionKind, string> = {
  pinned: 'New section',
  latest: 'Latest',
  most_opened: 'Most opened',
  recently_visited: 'Recently visited',
}

export function defaultSectionSettings(kind: DashboardSectionKind = 'pinned'): DashboardSectionSettings {
  return {
    kind,
    display: kind === 'pinned' ? 'medium' : 'small',
    item_limit: DASHBOARD_SECTION_DEFAULT_ITEMS,
    source: { category_id: null, tag: null, site: null, favorites: false, unread: false },
    open_window: 'all_time',
  }
}

/** Whether a section fills itself from the owner's library (anything but pinned). */
export function sectionFillsItself(section: Pick<DashboardSectionSettings, 'kind'>): boolean {
  return section.kind !== 'pinned'
}

export type DashboardDraft = {
  title: string
  sections: DashboardDraftSection[]
}

export function emptyDashboardDraft(): DashboardDraft {
  return {
    title: 'Home',
    sections: [{ ...defaultSectionSettings(), key: newClientId(), title: 'Favorites', bookmark_ids: [] }],
  }
}

export function newDraftSection(title?: string, kind: DashboardSectionKind = 'pinned'): DashboardDraftSection {
  return {
    ...defaultSectionSettings(kind),
    key: newClientId(),
    title: title ?? SECTION_KIND_TITLES[kind],
    bookmark_ids: [],
  }
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
  sections: (DashboardSectionSettings & { title: string; bookmark_ids: string[] })[]
} {
  return {
    title: draft.title.trim() || 'Home',
    sections: draft.sections.map((s) => ({
      title: s.title.trim(),
      // A section that fills itself has no pins of its own.
      bookmark_ids: sectionFillsItself(s) ? [] : s.bookmark_ids,
      kind: s.kind,
      display: s.display,
      item_limit: s.item_limit,
      source: {
        category_id: s.source.category_id || null,
        tag: s.source.tag?.trim() || null,
        site: s.source.site?.trim() || null,
        favorites: s.source.favorites,
        unread: s.source.unread,
      },
      open_window: s.open_window,
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
    if (sectionFillsItself(section)) continue
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

/**
 * One line saying what fills a section, such as "Newest · in Work · tagged rust · from
 * github.com · favorites". Null for a pinned section.
 */
export function describeSectionSource(
  section: DashboardSectionSettings,
  folderName: (id: string) => string | undefined
): string | null {
  if (!sectionFillsItself(section)) return null
  if (section.kind === 'recently_visited') return 'The bookmarks you opened last'
  const parts = [
    section.kind === 'latest'
      ? 'Newest'
      : section.open_window === 'last_30_days'
        ? 'Most opened in the last 30 days'
        : 'Most opened of all time',
  ]
  const { category_id, tag, site, favorites, unread } = section.source
  if (category_id) parts.push(`in ${folderName(category_id) ?? 'a deleted folder'}`)
  if (tag) parts.push(`tagged ${tag}`)
  if (site) parts.push(`from ${site}`)
  if (favorites) parts.push('favorites')
  if (unread) parts.push('unread')
  if (parts.length === 1) parts.push('whole library')
  return parts.join(' · ')
}
