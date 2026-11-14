import { describe, expect, it } from 'vitest'
import { oidcStartUrl, ssoButtons, ssoConnectUrl, TERMS_VERSION } from './terms'

describe('terms', () => {
  it('sends the current version when starting OIDC sign-in', () => {
    expect(oidcStartUrl()).toBe(`/api/auth/oidc/start?terms_version=${TERMS_VERSION}`)
    expect(oidcStartUrl('github')).toBe(`/api/auth/oidc/start?terms_version=${TERMS_VERSION}&provider=github`)
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
