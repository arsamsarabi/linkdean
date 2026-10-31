import { describe, expect, it } from 'vitest'
import { googleFavicon, hostname } from './urlDisplay'

describe('hostname', () => {
  it('strips www', () => {
    expect(hostname('https://www.example.com/path')).toBe('example.com')
  })

  it('returns raw string on invalid URL', () => {
    expect(hostname('not-a-url')).toBe('not-a-url')
  })
})

describe('googleFavicon', () => {
  it('builds favicon URL', () => {
    expect(googleFavicon('https://www.example.com')).toBe(
      'https://www.google.com/s2/favicons?domain=example.com&sz=64',
    )
  })
})
