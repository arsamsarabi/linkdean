import { describe, expect, it } from 'vitest'
import { beaconUrl, optedOut, routePath } from './goatcounter'

describe('GoatCounter page counting', () => {
  it('sends the route name, never ids', () => {
    expect(routePath('/_app/library')).toBe('/library')
    expect(routePath('/_app/bookmarks/$id')).toBe('/bookmarks/$id')
    expect(routePath('/_app/')).toBe('/')
    expect(routePath('/login')).toBe('/login')
  })

  it('builds the count URL with the host, the route and the screen size only', () => {
    const url = new URL(
      beaconUrl(
        'https://stats.example.com',
        'app.example.com',
        '/bookmarks/$id',
        { width: 1440, height: 900, devicePixelRatio: 2 },
        'abc'
      )
    )
    expect(url.origin + url.pathname).toBe('https://stats.example.com/count')
    const q = (k: string): string | null => url.searchParams.get(k)
    expect([q('p'), q('t'), q('s'), q('rnd')]).toEqual([
      'app.example.com/bookmarks/$id',
      'app.example.com/bookmarks/$id',
      '1440,900,2',
      'abc',
    ])
    expect(url.search.split('&')).toHaveLength(4)
  })

  it('counts nothing under Do Not Track or Global Privacy Control', () => {
    expect(optedOut({ doNotTrack: '1' })).toBe(true)
    expect(optedOut({ globalPrivacyControl: true })).toBe(true)
    expect(optedOut({ doNotTrack: null, globalPrivacyControl: false })).toBe(false)
  })
})
