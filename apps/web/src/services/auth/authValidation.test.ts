import { describe, expect, it } from 'vitest'
import { validatePasswordPair } from './authValidation'

describe('validatePasswordPair', () => {
  it('rejects short password', () => {
    expect(validatePasswordPair('short', 'short')).toEqual({
      ok: false,
      message: 'Password min 12 chars',
    })
  })

  it('rejects mismatch', () => {
    expect(validatePasswordPair('long-enough-pw', 'different')).toEqual({
      ok: false,
      message: 'Passwords mismatch',
    })
  })

  it('accepts matching pair with custom messages', () => {
    expect(
      validatePasswordPair('long-enough-pw', 'long-enough-pw', {
        tooShort: 'Vault passphrase min 12 chars',
        mismatch: 'Passphrases mismatch',
      })
    ).toEqual({ ok: true })
  })
})
