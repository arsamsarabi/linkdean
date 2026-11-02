import { describe, expect, it } from 'vitest'
import type { Bookmark, Dashboard } from '@/schemas'
import {
  dashboardDraftError,
  dashboardToShowInstead,
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
      sections: [{ key: '1', title: ' Keep ', bookmark_ids: ['b1'] }],
    }
    expect(toPutDashboardLayout(draft)).toEqual({
      title: 'Home',
      sections: [{ title: 'Keep', bookmark_ids: ['b1'] }],
    })
    expect(dashboardDraftError(draft)).toBeNull()
  })

  it('refuses a blank section title instead of dropping its pins', () => {
    const draft: DashboardDraft = {
      title: 'Home',
      sections: [
        { key: '1', title: 'Keep', bookmark_ids: ['b1'] },
        { key: '2', title: '   ', bookmark_ids: ['b2'] },
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
    sections: [{ items: [{ bookmark: pinned }, { bookmark: { ...mine, title: 'Stale' } }] }],
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
