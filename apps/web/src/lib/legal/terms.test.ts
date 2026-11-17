import { describe, expect, it } from 'vitest'
import { oidcStartUrl, ssoButtons, ssoConnectUrl, termsSatisfied } from './terms'

describe('terms', () => {
  it('sends the current version when starting OIDC sign-in', () => {
    expect(oidcStartUrl('2026-11-15')).toBe('/api/auth/oidc/start?terms_version=2026-11-15')
    expect(oidcStartUrl('2026-11-15', 'github')).toBe('/api/auth/oidc/start?terms_version=2026-11-15&provider=github')
    expect(oidcStartUrl(null, 'github')).toBe('/api/auth/oidc/start?provider=github')
    expect(oidcStartUrl(undefined)).toBe('/api/auth/oidc/start')
  })

  it('needs the box ticked only on a server with terms', () => {
    expect(termsSatisfied('2026-11-15', false)).toBe(false)
    expect(termsSatisfied('2026-11-15', true)).toBe(true)
    expect(termsSatisfied(null, false)).toBe(true)
  })

  it('names the provider to connect', () => {
    expect(ssoConnectUrl()).toBe('/api/auth/oidc/start?mode=connect')
    expect(ssoConnectUrl('google')).toBe('/api/auth/oidc/start?mode=connect&provider=google')
  })

  it("uses the server's providers, or one generic button for an older server", () => {
    const list = [{ id: 'google', label: 'Google' }]
    expect(ssoButtons(true, list)).toEqual(list)
    expect(ssoButtons(true, [])).toEqual([{ id: 'oidc', label: 'single sign-on' }])
    expect(ssoButtons(false, undefined)).toEqual([])
  })
})
