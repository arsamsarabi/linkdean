import { describe, expect, it } from 'vitest'
import {
  asStringList,
  asWebUrl,
  buildLibrarySearch,
  listToSearchParam,
  validateLibrarySearch,
  withWebScheme,
} from './librarySearch'

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
      create: undefined,
      focus: undefined,
    })
  })

  it('keeps folder except all', () => {
    expect(validateLibrarySearch({ folder: 'all' }).folder).toBeUndefined()
    expect(validateLibrarySearch({ folder: 'uncategorized' }).folder).toBe('uncategorized')
  })

  it('parses one-shot create and focus', () => {
    expect(validateLibrarySearch({ create: '1', focus: 'search' })).toMatchObject({
      create: true,
      focus: 'search',
    })
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
      create: undefined,
      focus: undefined,
    })
  })

  it('includes create and focus when set', () => {
    expect(buildLibrarySearch({ create: true, focus: 'search' })).toMatchObject({
      create: true,
      focus: 'search',
    })
  })
})

describe('asWebUrl', () => {
  it('accepts one http(s) address and refuses anything else', () => {
    expect(asWebUrl('  https://mantine.dev/core  ')).toBe('https://mantine.dev/core')
    expect(asWebUrl('http://localhost:3000/x?y=1')).toBe('http://localhost:3000/x?y=1')
    expect(asWebUrl('javascript:alert(1)')).toBeUndefined()
    expect(asWebUrl('ftp://example.com')).toBeUndefined()
    expect(asWebUrl('see https://example.com')).toBeUndefined()
    expect(asWebUrl('https://a.com https://b.com')).toBeUndefined()
    expect(asWebUrl('hello')).toBeUndefined()
    expect(asWebUrl(undefined)).toBeUndefined()
  })

  it('carries a pasted address through the library search, and drops a bad one', () => {
    expect(validateLibrarySearch({ addUrl: 'https://react.dev' }).addUrl).toBe('https://react.dev')
    expect(validateLibrarySearch({ addUrl: 'javascript:void 0' }).addUrl).toBeUndefined()
    expect(buildLibrarySearch({ addUrl: 'https://react.dev', folder: 'f1' })).toMatchObject({
      addUrl: 'https://react.dev',
      folder: 'f1',
    })
  })
})

describe('withWebScheme', () => {
  it('adds https to an address typed without a scheme', () => {
    expect(withWebScheme(' example.com/page ')).toBe('https://example.com/page')
    expect(withWebScheme('//example.com')).toBe('https://example.com')
    expect(withWebScheme('localhost:3000/x')).toBe('https://localhost:3000/x')
  })

  it('leaves an address with a scheme alone', () => {
    expect(withWebScheme('http://example.com')).toBe('http://example.com')
    expect(withWebScheme('javascript:alert(1)')).toBe('javascript:alert(1)')
    expect(withWebScheme('  ')).toBe('')
  })
})
