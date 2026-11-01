import { describe, expect, it } from 'vitest'
import type { Bookmark, Category } from '@/schemas'
import { categoriesForSelection, matchesVaultQuery, scopeBookmarks, scopeCategories } from './vaultScope'

function bookmark(patch: Partial<Bookmark>): Bookmark {
  return { title: '', url: '', description: null, tags: [], is_vault: false, ...patch } as Bookmark
}

describe('vaultScope', () => {
  const items = [
    bookmark({ id: 'plain' }),
    bookmark({ id: 'a', is_vault: true, vault_id: 'v1' }),
    bookmark({ id: 'b', is_vault: true, vault_id: 'v2' }),
  ]

  it('keeps only vault bookmarks', () => {
    expect(scopeBookmarks(items, null).map((b) => b.id)).toEqual(['a', 'b'])
  })

  it('narrows bookmarks to one vault', () => {
    expect(scopeBookmarks(items, 'v2').map((b) => b.id)).toEqual(['b'])
  })

  it('narrows categories the same way', () => {
    const cats = [
      { id: 'plain', is_vault: false },
      { id: 'a', is_vault: true, vault_id: 'v1' },
    ] as Category[]
    expect(scopeCategories(cats, null).map((c) => c.id)).toEqual(['a'])
    expect(scopeCategories(cats, 'v2')).toEqual([])
  })

  it('offers bulk categories from the selected vaults only', () => {
    const cats = [
      { id: 'one', vault_id: 'v1' },
      { id: 'two', vault_id: 'v2' },
    ] as Category[]
    expect(categoriesForSelection(cats, items, new Set(['a'])).map((c) => c.id)).toEqual(['one'])
    expect(categoriesForSelection(cats, items, new Set(['a', 'b'])).map((c) => c.id)).toEqual(['one', 'two'])
  })

  it('handles lists that have not loaded', () => {
    expect(scopeBookmarks(undefined, null)).toEqual([])
    expect(scopeCategories(undefined, null)).toEqual([])
  })

  it('searches title, url, description and tags', () => {
    const item = bookmark({ title: 'Rust', url: 'https://a.example/', description: 'Docs', tags: ['Lang'] })
    for (const q of ['rust', 'A.EXAMPLE', 'docs', 'lang']) expect(matchesVaultQuery(item, q)).toBe(true)
    expect(matchesVaultQuery(item, 'python')).toBe(false)
  })
})
