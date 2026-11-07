import { describe, expect, it } from 'vitest'
import { oidcStartUrl, TERMS_VERSION } from './terms'

describe('terms', () => {
  it('sends the current version when starting OIDC sign-in', () => {
    expect(oidcStartUrl()).toBe(`/api/auth/oidc/start?terms_version=${TERMS_VERSION}`)
  })
})
