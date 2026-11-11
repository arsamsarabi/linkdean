import { describe, expect, it } from 'vitest'
import { actionFor, changedFields, decisionsBody, tally } from './duplicateDecisions'

describe('duplicate decisions', () => {
  it('uses the hand-picked choice, else the default', () => {
    const picked = new Map([[3, 'replace' as const]])
    expect(actionFor(3, 'skip', picked)).toBe('replace')
    expect(actionFor(4, 'skip', picked)).toBe('skip')
  })

  it('sends only rows that differ from the default', () => {
    const picked = new Map([
      [3, 'replace' as const],
      [4, 'skip' as const],
    ])
    expect(decisionsBody('skip', picked)).toEqual({ default: 'skip', overrides: { 3: 'replace' } })
  })

  it('counts each action across every row, not just the visible page', () => {
    const picked = new Map([[2, 'keep' as const]])
    expect(tally([1, 2, 3], 'replace', picked)).toEqual({ skip: 0, replace: 2, merge: 0, keep: 1 })
  })

  it('lists only the fields the file would change', () => {
    const base = {
      index: 0,
      url: 'https://a.example',
      existing: { id: 'b', title: 'Mine', description: 'Notes', tags: ['a', 'b'], folders: ['Work'], updated_at: '' },
      incoming: { title: 'Mine ', description: null, tags: ['b', 'a'], folders: [] },
    }
    expect(changedFields(base)).toEqual([])
    expect(
      changedFields({ ...base, incoming: { title: 'Theirs', description: 'Other', tags: ['c'], folders: ['Home'] } })
    ).toEqual(['title', 'description', 'tags', 'folders'])
  })
})
