import { describe, expect, it } from 'vitest'
import type { Bookmark, Dashboard, DashboardSection } from '@/schemas'
import { defaultSectionSettings } from './dashboardDraft'
import {
  filterSections,
  greeting,
  isNew,
  itemMeta,
  layoutWithPin,
  relativeTime,
  sectionLibrarySearch,
} from './dashboardView'

const NOW = Date.parse('2026-11-13T12:00:00Z')
const ago = (ms: number): string => new Date(NOW - ms).toISOString()
const H = 3_600_000

const bm = (id: string, extra: Partial<Bookmark> = {}): Bookmark =>
  ({ id, title: id, url: `https://${id}.example/x`, tags: [], created_at: ago(2 * H), ...extra }) as Bookmark

const section = (kind: DashboardSection['kind'], items: Bookmark[], extra: Partial<DashboardSection> = {}) =>
  ({
    ...defaultSectionSettings(kind),
    id: `s-${kind}`,
    title: kind,
    position: 0,
    items: items.map((bookmark, i) => ({ id: bookmark.id, position: i, bookmark })),
    ...extra,
  }) as DashboardSection

describe('relativeTime', () => {
  it('reads like a person would say it', () => {
    expect(relativeTime(ago(10_000), NOW)).toBe('just now')
    expect(relativeTime(ago(5 * 60_000), NOW)).toBe('5m ago')
    expect(relativeTime(ago(3 * H), NOW)).toBe('3h ago')
    expect(relativeTime(ago(50 * H), NOW)).toBe('2d ago')
    expect(relativeTime(ago(15 * 24 * H), NOW)).toBe('2w ago')
    expect(relativeTime('nonsense', NOW)).toBe('')
  })

  it('marks bookmarks saved in the last day as new', () => {
    expect(isNew(ago(23 * H), NOW)).toBe(true)
    expect(isNew(ago(25 * H), NOW)).toBe(false)
  })
})

describe('itemMeta', () => {
  it('ranks most opened and counts opens', () => {
    expect(itemMeta({ kind: 'most_opened' }, { bookmark: bm('a'), open_count: 1 }, 0, NOW)).toEqual({
      text: '1 open',
      rank: 1,
      fresh: false,
    })
    expect(itemMeta({ kind: 'most_opened' }, { bookmark: bm('a'), open_count: 12 }, 2, NOW).text).toBe('12 opens')
  })

  it('says when a bookmark was opened or saved, and nothing for pins', () => {
    expect(itemMeta({ kind: 'recently_visited' }, { bookmark: bm('a'), last_opened_at: ago(2 * H) }, 0, NOW).text).toBe(
      '2h ago'
    )
    expect(itemMeta({ kind: 'latest' }, { bookmark: bm('a') }, 0, NOW)).toEqual({
      text: 'Saved 2h ago',
      rank: null,
      fresh: true,
    })
    expect(itemMeta({ kind: 'pinned' }, { bookmark: bm('a') }, 0, NOW).text).toBeNull()
  })
})

describe('sectionLibrarySearch', () => {
  it('maps a section source onto library filters', () => {
    const latest = section('latest', [], {
      source: { category_id: 'f1', tag: 'rust', site: 'github.com', favorites: true, unread: true },
    })
    expect(sectionLibrarySearch(latest)).toMatchObject({
      q: 'github.com',
      sort: 'created_desc',
      category: 'f1',
      tag: 'rust',
      flag: ['favorite', 'unread'],
    })
  })

  it('has nothing for pinned and recently visited sections', () => {
    expect(sectionLibrarySearch(section('pinned', []))).toBeNull()
    expect(sectionLibrarySearch(section('recently_visited', []))).toBeNull()
  })
})

describe('filterSections', () => {
  const sections = [
    section('pinned', [bm('github', { title: 'GitHub' }), bm('mdn', { title: 'MDN', tags: ['docs'] })]),
    section('latest', [bm('crates', { title: 'Rust crates' })]),
  ]

  it('keeps every section and only the matching bookmarks', () => {
    const out = filterSections(sections, '  DOCS ')
    expect(out.map((s) => s.items.map((i) => i.bookmark.id))).toEqual([['mdn'], []])
    expect(filterSections(sections, 'rust')[1]!.items).toHaveLength(1)
    expect(filterSections(sections, 'github.example')[0]!.items).toHaveLength(1)
  })

  it('returns the sections as they are without a filter', () => {
    expect(filterSections(sections, ' ')).toBe(sections)
  })
})

describe('layoutWithPin', () => {
  const dashboard = {
    title: 'Home',
    sections: [section('pinned', [bm('a')]), section('latest', [bm('b')])],
  } as Dashboard

  it('adds the bookmark to the end of that section only', () => {
    const out = layoutWithPin(dashboard, 's-pinned', 'c')
    expect(out.sections[0]!.bookmark_ids).toEqual(['a', 'c'])
    expect(out.sections[1]).toMatchObject({ kind: 'latest', bookmark_ids: [] })
  })

  it('leaves a bookmark that is already pinned where it is', () => {
    expect(layoutWithPin(dashboard, 's-pinned', 'a').sections[0]!.bookmark_ids).toEqual(['a'])
  })
})

describe('greeting', () => {
  it('follows the local hour', () => {
    expect(greeting(new Date(2026, 10, 13, 8))).toBe('Good morning')
    expect(greeting(new Date(2026, 10, 13, 14))).toBe('Good afternoon')
    expect(greeting(new Date(2026, 10, 13, 21))).toBe('Good evening')
    expect(greeting(new Date(2026, 10, 13, 2))).toBe('Good evening')
  })
})
