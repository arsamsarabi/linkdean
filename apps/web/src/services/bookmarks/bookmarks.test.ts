import { describe, expect, it } from 'vitest'
import type { Bookmark } from '@/schemas'
import { draftFromBookmark, emptyDraft } from './bookmarks'

describe('bookmarks drafts', () => {
  it('emptyDraft returns blank fields', () => {
    expect(emptyDraft()).toEqual({
      url: '',
      title: '',
      description: '',
      tags: [],
      category_ids: [],
    })
  })

  it('draftFromBookmark maps categories and fields', () => {
    const b: Bookmark = {
      id: 'b1',
      url: 'https://example.com',
      title: 'Ex',
      description: 'd',
      notes: null,
      favicon_url: null,
      preview_image_url: null,
      has_preview: false,
      is_vault: false,
      tags: ['t'],
      categories: [{ id: 'c1', name: 'C', path: 'C' }],
      category_id: 'c1',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    }
    expect(draftFromBookmark(b)).toEqual({
      id: 'b1',
      url: 'https://example.com',
      title: 'Ex',
      description: 'd',
      tags: ['t'],
      category_ids: ['c1'],
    })
  })
})
