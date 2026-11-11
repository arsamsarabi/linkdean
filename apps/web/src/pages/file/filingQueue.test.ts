import { describe, expect, it } from 'vitest'
import type { Bookmark } from '@/schemas'
import { currentCard, leftToFile, queueKey, toggle } from './filingQueue'

const b = (id: string): Bookmark => ({ id }) as Bookmark

describe('filing queue helpers', () => {
  it('takes the first bookmark not yet handled', () => {
    const items = [b('a'), b('b'), b('c')]
    expect(currentCard(items, new Set())?.id).toBe('a')
    expect(currentCard(items, new Set(['a', 'b']))?.id).toBe('c')
    expect(currentCard(items, new Set(['a', 'b', 'c']))).toBeUndefined()
  })

  it('counts what is left, taking off skipped ones still in the list', () => {
    expect(leftToFile(10, [b('a'), b('b')], new Set(['a']))).toBe(9)
    // A filed one has already left both the list and the total.
    expect(leftToFile(9, [b('b')], new Set(['a']))).toBe(9)
    expect(leftToFile(0, [], new Set(['a']))).toBe(0)
  })

  it('maps keys on the page, in the title and in other fields', () => {
    expect(queueKey({ key: 'Enter' }, 'page')).toEqual({ kind: 'file' })
    expect(queueKey({ key: 'S' }, 'page')).toEqual({ kind: 'skip' })
    expect(queueKey({ key: 'd' }, 'page')).toEqual({ kind: 'delete' })
    expect(queueKey({ key: '3' }, 'page')).toEqual({ kind: 'folder', index: 2 })
    expect(queueKey({ key: '6' }, 'page')).toBeNull()
    expect(queueKey({ key: 's', metaKey: true }, 'page')).toBeNull()
    expect(queueKey({ key: 'Enter' }, 'title')).toEqual({ kind: 'file' })
    expect(queueKey({ key: 's' }, 'title')).toBeNull()
    expect(queueKey({ key: 'Enter' }, 'field')).toBeNull()
  })

  it('toggles an id', () => {
    expect(toggle(['a'], 'b')).toEqual(['a', 'b'])
    expect(toggle(['a', 'b'], 'a')).toEqual(['b'])
  })
})
