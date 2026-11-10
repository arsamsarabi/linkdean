import { describe, expect, it } from 'vitest'
import { challengeFor, randomState, randomVerifier } from './pkce'

describe('PKCE', () => {
  it('matches the RFC 7636 example', async () => {
    expect(await challengeFor('dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk')).toBe(
      'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM'
    )
  })

  it('makes verifiers the server accepts', () => {
    const v = randomVerifier()
    expect(v).toMatch(/^[A-Za-z0-9_-]{43}$/)
    expect(randomVerifier()).not.toBe(v)
    expect(randomState()).toMatch(/^[A-Za-z0-9_-]{22}$/)
  })
})
