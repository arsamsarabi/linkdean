import { describe, expect, it } from 'vitest'
import type { Bookmark } from '@/schemas'
import { mergeMeta, pickDefaultKeepId } from './mergeDuplicates'

function bm(partial: Partial<Bookmark> & Pick<Bookmark, 'id' | 'updated_at'>): Bookmark {
  return {
    owner_user_id: 'u',
    categories: [],
    url: 'https://ex.com',
    title: 't',
    has_preview: false,
    is_vault: false,
    tags: [],
    can_write: true,
    shared_out: false,
    created_at: partial.updated_at,
    ...partial,
  }
}

describe('pickDefaultKeepId', () => {
  it('picks newest updated', () => {
    expect(
      pickDefaultKeepId([
        bm({ id: 'a', updated_at: '2024-01-01T00:00:00Z' }),
        bm({ id: 'b', updated_at: '2025-01-01T00:00:00Z' }),
      ])
    ).toBe('b')
  })
})

describe('mergeMeta', () => {
  it('unions tags and categories', () => {
    const keep = bm({
      id: 'a',
      updated_at: '2025-01-01T00:00:00Z',
      tags: ['x'],
      categories: [{ id: 'c1', name: 'A', path: 'A' }],
    })
    const loser = bm({
      id: 'b',
      updated_at: '2024-01-01T00:00:00Z',
      tags: ['x', 'y'],
      categories: [{ id: 'c2', name: 'B', path: 'B' }],
    })
    expect(mergeMeta(keep, [loser])).toEqual({
      tags: ['x', 'y'],
      category_ids: ['c1', 'c2'],
    })
  })
})
