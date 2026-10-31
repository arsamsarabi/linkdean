import { describe, expect, it } from 'vitest'
import { resolveDefaultLanding } from './defaultLanding'

describe('resolveDefaultLanding', () => {
  it('maps library and falls back to dashboard', () => {
    expect(resolveDefaultLanding('/library')).toBe('/library')
    expect(resolveDefaultLanding('/')).toBe('/')
    expect(resolveDefaultLanding('/vault')).toBe('/')
    expect(resolveDefaultLanding('/import')).toBe('/')
    expect(resolveDefaultLanding(undefined)).toBe('/')
  })
})
