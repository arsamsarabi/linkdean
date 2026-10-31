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
    })
  })
})

describe('buildLibrarySearch', () => {
  it('builds search object', () => {
    expect(buildLibrarySearch({ q: 'x', sort: 'title_asc', category: ['c'], tag: [] })).toEqual({
      q: 'x',
      sort: 'title_asc',
      category: 'c',
      tag: undefined,
    })
  })
})
