import { describe, expect, it } from 'vitest'
import { validatePasswordPair } from './authValidation'

describe('validatePasswordPair', () => {
  it('rejects short password', () => {
    expect(validatePasswordPair('short', 'short')).toEqual({
      ok: false,
      message: 'Password min 8 chars',
    })
  })

  it('rejects mismatch', () => {
    expect(validatePasswordPair('longenough', 'different')).toEqual({
      ok: false,
      message: 'Passwords mismatch',
    })
  })

  it('accepts matching pair with custom messages', () => {
    expect(
      validatePasswordPair('longenough', 'longenough', {
        tooShort: 'Vault passphrase min 8 chars',
        mismatch: 'Passphrases mismatch',
      }),
    ).toEqual({ ok: true })
  })
})
