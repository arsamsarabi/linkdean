import { describe, expect, it } from 'vitest'
import { moveSection, toPutDashboardLayout, type DashboardDraft } from './dashboardDraft'

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
  it('drops blank titles and defaults dashboard title', () => {
    const draft: DashboardDraft = {
      title: '  ',
      sections: [
        { key: '1', title: ' Keep ', bookmark_ids: ['b1'] },
        { key: '2', title: '   ', bookmark_ids: ['b2'] },
      ],
    }
    expect(toPutDashboardLayout(draft)).toEqual({
      title: 'Home',
      sections: [{ title: 'Keep', bookmark_ids: ['b1'] }],
    })
  })
})
