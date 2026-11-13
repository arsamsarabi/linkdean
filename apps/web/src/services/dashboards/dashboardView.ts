import { buildLibrarySearch, type LibrarySearch } from '@/lib/library/librarySearch'
import type { Dashboard, DashboardItem, DashboardSection, PutDashboardLayout } from '@/schemas'
import { hostname } from '@/services/url-display/urlDisplay'
import { sectionFillsItself, toPutDashboardLayout, type DashboardDraft } from './dashboardDraft'

const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

/** "just now", "5m ago", "3h ago", "2d ago", "3w ago", then a short date. */
export function relativeTime(iso: string, now: number = Date.now()): string {
  const then = Date.parse(iso)
  if (Number.isNaN(then)) return ''
  const diff = Math.max(0, now - then)
  if (diff < MINUTE) return 'just now'
  if (diff < HOUR) return `${Math.floor(diff / MINUTE)}m ago`
  if (diff < DAY) return `${Math.floor(diff / HOUR)}h ago`
  if (diff < 7 * DAY) return `${Math.floor(diff / DAY)}d ago`
  if (diff < 35 * DAY) return `${Math.floor(diff / (7 * DAY))}w ago`
  return new Date(then).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
}

/** Saved in the last 24 hours. */
export function isNew(iso: string, now: number = Date.now()): boolean {
  const then = Date.parse(iso)
  return !Number.isNaN(then) && now - then < DAY
}

export type ItemMeta = {
  /** Short text shown beside or under the bookmark, such as "12 opens" or "2h ago". */
  text: string | null
  /** 1-based rank, for Most opened. */
  rank: number | null
  /** Saved in the last day, for Latest. */
  fresh: boolean
}

/** What a section adds to each of its bookmarks: rank and opens, when it was opened, or when saved. */
export function itemMeta(
  section: Pick<DashboardSection, 'kind'>,
  item: Pick<DashboardItem, 'open_count' | 'last_opened_at' | 'bookmark'>,
  index: number,
  now: number = Date.now()
): ItemMeta {
  switch (section.kind) {
    case 'most_opened': {
      const n = item.open_count ?? 0
      return { text: `${n} ${n === 1 ? 'open' : 'opens'}`, rank: index + 1, fresh: false }
    }
    case 'recently_visited':
      return {
        text: item.last_opened_at ? relativeTime(item.last_opened_at, now) : null,
        rank: null,
        fresh: false,
      }
    case 'latest':
      return {
        text: `Saved ${relativeTime(item.bookmark.created_at, now)}`,
        rank: null,
        fresh: isNew(item.bookmark.created_at, now),
      }
    default:
      return { text: null, rank: null, fresh: false }
  }
}

/**
 * The library view closest to a section, for its "See all in library" link. Null when the
 * library has no equivalent (Recently visited, or a pinned section).
 */
export function sectionLibrarySearch(section: DashboardSection): LibrarySearch | null {
  if (section.kind !== 'latest' && section.kind !== 'most_opened') return null
  const { category_id, tag, site, favorites, unread } = section.source
  const flag = [...(favorites ? ['favorite'] : []), ...(unread ? ['unread'] : [])]
  return buildLibrarySearch({
    q: site || undefined,
    sort: section.kind === 'latest' ? 'created_desc' : undefined,
    category: category_id ? [category_id] : [],
    tag: tag ? [tag] : [],
    flag,
  })
}

function matches(item: DashboardItem, needle: string): boolean {
  const b = item.bookmark
  return (
    b.title.toLowerCase().includes(needle) ||
    b.url.toLowerCase().includes(needle) ||
    hostname(b.url).toLowerCase().includes(needle) ||
    (b.tags ?? []).some((t) => t.toLowerCase().includes(needle))
  )
}

/** Every section with only the bookmarks matching the filter text (title, address, site, tag). */
export function filterSections(sections: DashboardSection[], query: string): DashboardSection[] {
  const needle = query.trim().toLowerCase()
  if (!needle) return sections
  return sections.map((s) => ({ ...s, items: s.items.filter((i) => matches(i, needle)) }))
}

/** The dashboard as an edit draft: pinned sections keep their pins, the others their settings. */
export function draftFromDashboard(d: Dashboard): DashboardDraft {
  return {
    title: d.title,
    sections: d.sections.map((s) => ({
      key: s.id,
      title: s.title,
      bookmark_ids: sectionFillsItself(s) ? [] : s.items.map((i) => i.bookmark.id),
      kind: s.kind,
      display: s.display,
      item_limit: s.item_limit,
      source: { ...s.source },
      open_window: s.open_window,
    })),
  }
}

/** The layout with one more bookmark pinned at the end of a section. Already there: unchanged. */
export function layoutWithPin(d: Dashboard, sectionId: string, bookmarkId: string): PutDashboardLayout {
  const draft = draftFromDashboard(d)
  draft.sections = draft.sections.map((s) =>
    s.key === sectionId && !s.bookmark_ids.includes(bookmarkId)
      ? { ...s, bookmark_ids: [...s.bookmark_ids, bookmarkId] }
      : s
  )
  return toPutDashboardLayout(draft)
}

/** "Good morning", "Good afternoon" or "Good evening" for the local hour. */
export function greeting(date: Date = new Date()): string {
  const h = date.getHours()
  if (h < 5) return 'Good evening'
  if (h < 12) return 'Good morning'
  if (h < 18) return 'Good afternoon'
  return 'Good evening'
}
