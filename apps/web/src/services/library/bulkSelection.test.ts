import { describe, expect, it } from 'vitest'
import { toggleId, unionTags, writableIds } from './bulkSelection'

describe('writableIds', () => {
  it('drops read-only and keeps default-writable', () => {
    expect(
      writableIds([
        { id: 'a' },
        { id: 'b', can_write: false },
        { id: 'c', can_write: true },
      ])
    ).toEqual(['a', 'c'])
  })
})

describe('toggleId', () => {
  it('adds and removes', () => {
    const once = toggleId(new Set(['a']), 'b')
    expect([...once].sort()).toEqual(['a', 'b'])
    expect([...toggleId(once, 'a')]).toEqual(['b'])
  })
})

describe('unionTags', () => {
  it('merges without dupes', () => {
    expect(unionTags(['a', 'b'], [' b ', 'c', ''])).toEqual(['a', 'b', 'c'])
  })
})
