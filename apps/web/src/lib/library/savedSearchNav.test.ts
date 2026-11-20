import { describe, expect, it } from 'vitest'
import { filtersFromLibraryState, librarySearchFromSavedFilters, pathForSavedFilters } from './savedSearchNav'

describe('savedSearchNav', () => {
  it('round-trips a typical filter set', () => {
    const filters = filtersFromLibraryState({
      q: 'rust',
      sort: 'title_asc',
      category: ['c1'],
      tag: ['docs'],
      flag: ['favorite'],
      since: '7d',
      vault: false,
      vaultId: null,
      folder: 'all',
    })
    expect(filters.folder).toBeUndefined()
    expect(librarySearchFromSavedFilters(filters)).toMatchObject({
      q: 'rust',
      sort: 'title_asc',
      category: 'c1',
      tag: 'docs',
      flag: 'favorite',
      since: '7d',
    })
    expect(pathForSavedFilters(filters)).toBe('/library')
  })

  it('routes vault filters to /vaults', () => {
    expect(pathForSavedFilters({ vault: true, category: [], tag: [], flag: [] })).toBe('/vaults')
    expect(pathForSavedFilters({ vault_id: 'v1', category: [], tag: [], flag: [] })).toBe('/vaults')
  })
})
