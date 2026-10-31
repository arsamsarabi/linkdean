import { describe, expect, it } from 'vitest'
import { asStringList, buildLibrarySearch, listToSearchParam, validateLibrarySearch } from './librarySearch'

describe('asStringList', () => {
  it('normalizes string, array, empty', () => {
    expect(asStringList('a,b')).toEqual(['a', 'b'])
    expect(asStringList(['a', ''])).toEqual(['a'])
    expect(asStringList(null)).toEqual([])
  })
})

describe('listToSearchParam', () => {
  it('omits empty, unwraps single', () => {
    expect(listToSearchParam([])).toBeUndefined()
    expect(listToSearchParam(['a'])).toBe('a')
    expect(listToSearchParam(['a', 'b'])).toEqual(['a', 'b'])
  })
})

describe('validateLibrarySearch', () => {
  it('drops default sort', () => {
    expect(validateLibrarySearch({ sort: 'updated_desc', q: ' hi ' })).toEqual({
      q: ' hi ',
      sort: undefined,
      category: undefined,
      tag: undefined,
      folder: undefined,
    })
  })

  it('keeps folder except all', () => {
    expect(validateLibrarySearch({ folder: 'all' }).folder).toBeUndefined()
    expect(validateLibrarySearch({ folder: 'uncategorized' }).folder).toBe('uncategorized')
  })
})

describe('buildLibrarySearch', () => {
  it('builds search object', () => {
    expect(buildLibrarySearch({ q: 'x', sort: 'title_asc', category: ['c'], tag: [], folder: 'root' })).toEqual({
      q: 'x',
      sort: 'title_asc',
      category: 'c',
      tag: undefined,
      folder: 'root',
    })
  })
})
