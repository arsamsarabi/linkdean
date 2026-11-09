import { describe, expect, it } from 'vitest'
import type { Bookmark, Category } from '@/schemas'
import { draftFromBookmark, draftHeadsToVault, emptyDraft, fillFromPage, isFetchableUrl } from './bookmarks'

describe('bookmarks drafts', () => {
  it('emptyDraft returns blank fields', () => {
    expect(emptyDraft()).toEqual({
      url: '',
      title: '',
      description: '',
      tags: [],
      category_ids: [],
      is_favorite: false,
      is_private: false,
      is_unread: false,
      is_pinned: false,
    })
  })

  it('draftFromBookmark maps categories and fields', () => {
    const b: Bookmark = {
      id: 'b1',
      owner_user_id: 'u1',
      url: 'https://example.com',
      title: 'Ex',
      description: 'd',
      notes: null,
      favicon_url: null,
      preview_image_url: null,
      has_preview: false,
      is_vault: false,
      tags: ['t'],
      can_write: true,
      shared_out: false,
      shared_with: [],
      is_favorite: true,
      is_private: false,
      is_unread: true,
      is_pinned: false,
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
      is_favorite: true,
      is_private: false,
      is_unread: true,
      is_pinned: false,
      has_preview: false,
      preview_image_url: null,
      updated_at: '2024-01-01T00:00:00Z',
      can_write: true,
      is_vault: false,
      owner_user_id: 'u1',
    })
  })
})

describe('filling a draft from the page', () => {
  const details = { title: 'Page title', description: 'Page text', tags: ['a', 'b'] }

  it('fills only empty fields', () => {
    const typed = { ...emptyDraft(), title: 'Mine', tags: ['keep'] }
    expect(fillFromPage(typed, details)).toMatchObject({ title: 'Mine', description: 'Page text', tags: ['keep'] })
    expect(fillFromPage(emptyDraft(), details)).toMatchObject({
      title: 'Page title',
      description: 'Page text',
      tags: ['a', 'b'],
    })
  })

  it('treats whitespace as empty and leaves fields alone when the page has nothing', () => {
    const spaced = { ...emptyDraft(), title: '   ' }
    expect(fillFromPage(spaced, details).title).toBe('Page title')
    expect(fillFromPage(spaced, { title: null, description: null, tags: [] }).title).toBe('   ')
  })

  it('knows when a draft is going into a vault', () => {
    const vaultCat = { id: 'v', is_vault: true } as Category
    const plainCat = { id: 'p', is_vault: false } as Category
    expect(draftHeadsToVault(emptyDraft(), [])).toBe(false)
    expect(draftHeadsToVault(emptyDraft(), [], true)).toBe(true)
    expect(draftHeadsToVault({ ...emptyDraft(), vault_id: 'x' }, [])).toBe(true)
    expect(draftHeadsToVault({ ...emptyDraft(), category_ids: ['p'] }, [plainCat, vaultCat])).toBe(false)
    expect(draftHeadsToVault({ ...emptyDraft(), category_ids: ['p', 'v'] }, [plainCat, vaultCat])).toBe(true)
  })

  it('only fetches web addresses', () => {
    expect(isFetchableUrl('https://example.com/a')).toBe(true)
    expect(isFetchableUrl('http://example.com')).toBe(true)
    expect(isFetchableUrl('https://')).toBe(false)
    expect(isFetchableUrl('javascript:alert(1)')).toBe(false)
    expect(isFetchableUrl('example.com')).toBe(false)
  })
})
