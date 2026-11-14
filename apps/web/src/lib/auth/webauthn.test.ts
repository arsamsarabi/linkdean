import { describe, expect, it } from 'vitest'
import { base64urlToBuffer, bufferToBase64url, isPasskeyCancel, toCreationOptions, toRequestOptions } from './webauthn'

describe('webauthn helpers', () => {
  it('round-trips unpadded base64url, including the URL-safe characters', () => {
    const bytes = new Uint8Array([251, 255, 0, 62, 63, 1, 2])
    const s = bufferToBase64url(bytes.buffer)
    expect(s).not.toMatch(/[+/=]/)
    expect(new Uint8Array(base64urlToBuffer(s))).toEqual(bytes)
  })

  it('turns the creation options binary fields into buffers', () => {
    const opts = toCreationOptions({
      publicKey: {
        challenge: 'AQID',
        rp: { id: 'localhost', name: 'Linkdean' },
        user: { id: 'BAUG', name: 'me@example.com', displayName: 'Me' },
        excludeCredentials: [{ id: 'BwgJ', type: 'public-key' }],
      },
    })
    const pk = opts.publicKey!
    expect(new Uint8Array(pk.challenge as ArrayBuffer)).toEqual(new Uint8Array([1, 2, 3]))
    expect(new Uint8Array(pk.user.id as ArrayBuffer)).toEqual(new Uint8Array([4, 5, 6]))
    expect(new Uint8Array(pk.excludeCredentials![0].id as ArrayBuffer)).toEqual(new Uint8Array([7, 8, 9]))
    expect(pk.rp.name).toBe('Linkdean')
  })

  it('turns the request options challenge into a buffer and keeps an empty allow list out', () => {
    const opts = toRequestOptions({ publicKey: { challenge: 'AQID', rpId: 'localhost', userVerification: 'required' } })
    expect(new Uint8Array(opts.publicKey!.challenge as ArrayBuffer)).toEqual(new Uint8Array([1, 2, 3]))
    expect(opts.publicKey!.allowCredentials).toBeUndefined()
  })

  it('treats a closed passkey dialog as a cancel, not an error', () => {
    expect(isPasskeyCancel(new DOMException('no', 'NotAllowedError'))).toBe(true)
    expect(isPasskeyCancel(new Error('boom'))).toBe(false)
  })
})
