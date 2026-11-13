import { describe, expect, it } from 'vitest'
import type { Bookmark, Dashboard } from '@/schemas'
import {
  dashboardDraftError,
  dashboardToShowInstead,
  defaultSectionSettings,
  describeSectionSource,
  mergeDashboardBookmarks,
  moveSection,
  toPutDashboardLayout,
  type DashboardDraft,
} from './dashboardDraft'

describe('moveSection', () => {
  it('swaps adjacent sections up and down', () => {
    const sections = ['a', 'b', 'c']
    expect(moveSection(sections, 1, -1)).toEqual(['b', 'a', 'c'])
    expect(moveSection(sections, 1, 1)).toEqual(['a', 'c', 'b'])
  })

  it('no-ops at edges', () => {
    const sections = ['a', 'b']
    expect(moveSection(sections, 0, -1)).toEqual(['a', 'b'])
    expect(moveSection(sections, 1, 1)).toEqual(['a', 'b'])
  })
})

describe('toPutDashboardLayout', () => {
  it('trims titles and defaults the dashboard title', () => {
    const draft: DashboardDraft = {
      title: '  ',
      sections: [{ ...defaultSectionSettings(), key: '1', title: ' Keep ', bookmark_ids: ['b1'] }],
    }
    expect(toPutDashboardLayout(draft)).toEqual({
      title: 'Home',
      sections: [
        {
          title: 'Keep',
          bookmark_ids: ['b1'],
          kind: 'pinned',
          display: 'medium',
          item_limit: 8,
          source: { category_id: null, tag: null, site: null, favorites: false, unread: false },
          open_window: 'all_time',
        },
      ],
    })
    expect(dashboardDraftError(draft)).toBeNull()
  })

  it('sends a self-filling section with its source and no pins', () => {
    const draft: DashboardDraft = {
      title: 'Home',
      sections: [
        {
          ...defaultSectionSettings('latest'),
          key: '1',
          title: 'Latest',
          bookmark_ids: ['stale'],
          source: { category_id: '', tag: ' rust ', site: ' github.com ', favorites: true, unread: false },
        },
      ],
    }
    expect(toPutDashboardLayout(draft).sections[0]).toMatchObject({
      kind: 'latest',
      display: 'small',
      bookmark_ids: [],
      source: { category_id: null, tag: 'rust', site: 'github.com', favorites: true, unread: false },
    })
  })

  it('refuses a blank section title instead of dropping its pins', () => {
    const draft: DashboardDraft = {
      title: 'Home',
      sections: [
        { ...defaultSectionSettings(), key: '1', title: 'Keep', bookmark_ids: ['b1'] },
        { ...defaultSectionSettings(), key: '2', title: '   ', bookmark_ids: ['b2'] },
      ],
    }
    expect(dashboardDraftError(draft)).toBe('Every section needs a title.')
    expect(toPutDashboardLayout(draft).sections).toHaveLength(2)
  })
})

describe('mergeDashboardBookmarks', () => {
  const mine = { id: 'mine', title: 'Mine', url: 'https://mine.test' } as Bookmark
  const pinned = { id: 'pinned', title: 'Pinned', url: 'https://pinned.test' } as Bookmark
  const dashboard = {
    sections: [{ kind: 'pinned', items: [{ bookmark: pinned }, { bookmark: { ...mine, title: 'Stale' } }] }],
  } as unknown as Dashboard

  it('adds pins missing from the library and keeps library copies', () => {
    expect(mergeDashboardBookmarks([mine], dashboard)).toEqual([mine, pinned])
  })

  it('returns the library when no dashboard has loaded', () => {
    expect(mergeDashboardBookmarks([mine], undefined)).toEqual([mine])
  })
})

describe('dashboardToShowInstead', () => {
  const list = [
    { id: 'other', is_default: false },
    { id: 'home', is_default: true },
  ]

  it('stays put while the dashboard is still listed', () => {
    expect(dashboardToShowInstead(list, 'other')).toBeUndefined()
  })

  it('moves to the default when the dashboard is gone', () => {
    expect(dashboardToShowInstead(list, 'revoked')).toBe('home')
  })

  it('uses the first dashboard when none is default', () => {
    expect(dashboardToShowInstead([{ id: 'other', is_default: false }], 'revoked')).toBe('other')
  })
})

describe('describeSectionSource', () => {
  const folders = (id: string): string | undefined => (id === 'f1' ? 'Work / Rust' : undefined)

  it('says nothing for a pinned section', () => {
    expect(describeSectionSource(defaultSectionSettings(), folders)).toBeNull()
  })

  it('lists every filter of a section that fills itself', () => {
    const latest = {
      ...defaultSectionSettings('latest'),
      source: { category_id: 'f1', tag: 'rust', site: 'github.com', favorites: true, unread: true },
    }
    expect(describeSectionSource(latest, folders)).toBe(
      'Newest · in Work / Rust · tagged rust · from github.com · favorites · unread'
    )
  })

  it('names the whole library, the counting window and a deleted folder', () => {
    expect(describeSectionSource(defaultSectionSettings('latest'), folders)).toBe('Newest · whole library')
    const opened = {
      ...defaultSectionSettings('most_opened'),
      open_window: 'last_30_days' as const,
      source: { ...defaultSectionSettings().source, category_id: 'gone' },
    }
    expect(describeSectionSource(opened, folders)).toBe('Most opened in the last 30 days · in a deleted folder')
    expect(describeSectionSource(defaultSectionSettings('recently_visited'), folders)).toBe(
      'The bookmarks you opened last'
    )
  })
})
