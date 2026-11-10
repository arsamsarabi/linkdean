import { describe, expect, it } from 'vitest'
import { authorizeUrl, codeFrom } from './signIn'

const redirect = `https://${'a'.repeat(32)}.chromiumapp.org/`

describe('sign-in addresses', () => {
  it('sends the request to the instance approve page', () => {
    const url = new URL(authorizeUrl('https://app.linkdean.app', redirect, 'chal', 'st'))
    expect(url.origin + url.pathname).toBe('https://app.linkdean.app/connect/extension')
    expect(Object.fromEntries(url.searchParams)).toEqual({
      redirect_uri: redirect,
      code_challenge: 'chal',
      code_challenge_method: 'S256',
      state: 'st',
    })
  })

  it('takes the code only when the state matches', () => {
    expect(codeFrom(`${redirect}?code=c1&state=st`, 'st')).toBe('c1')
    expect(() => codeFrom(`${redirect}?code=c1&state=other`, 'st')).toThrow(/did not match/)
    expect(() => codeFrom(`${redirect}?error=access_denied&state=st`, 'st')).toThrow(/cancelled/)
    expect(() => codeFrom(`${redirect}?state=st`, 'st')).toThrow(/code/)
  })
})
