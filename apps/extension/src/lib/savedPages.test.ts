import { describe, expect, it } from 'vitest'
import { cacheFresh, HASHES_MAX_AGE_MS, urlHash } from './savedPages'

describe('saved page checks', () => {
  it('hashes an address the way the server does', async () => {
    // Same vector as the API's own test: SHA-256 of the exact URL, first 16 hex digits.
    expect(await urlHash('https://saved.example/page')).toBe('d41412cfea200948')
    expect(await urlHash(' https://saved.example/page ')).toBe('d41412cfea200948')
    expect(await urlHash('https://saved.example/page/')).not.toBe('d41412cfea200948')
  })

  it('reuses a downloaded list only for the same instance and while it is fresh', () => {
    const cache = { instance: 'https://a.example', hashes: [], fetchedAt: 1_000 }
    expect(cacheFresh(cache, 'https://a.example', 1_000 + HASHES_MAX_AGE_MS - 1)).toBe(true)
    expect(cacheFresh(cache, 'https://a.example', 1_000 + HASHES_MAX_AGE_MS)).toBe(false)
    expect(cacheFresh(cache, 'https://b.example', 1_001)).toBe(false)
    expect(cacheFresh(null, 'https://a.example', 1_001)).toBe(false)
  })
})
