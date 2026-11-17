import { describe, expect, it } from 'vitest'
import { hostname } from './urlDisplay'

describe('hostname', () => {
  it('strips www', () => {
    expect(hostname('https://www.example.com/path')).toBe('example.com')
  })

  it('returns raw string on invalid URL', () => {
    expect(hostname('not-a-url')).toBe('not-a-url')
  })
})
