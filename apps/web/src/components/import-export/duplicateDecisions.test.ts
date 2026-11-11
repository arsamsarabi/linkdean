import { describe, expect, it } from 'vitest'
import { actionFor, decisionsBody } from './duplicateDecisions'

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
})
