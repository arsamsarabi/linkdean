/**
 * Browser side of passkeys. The API sends WebAuthn options as JSON with binary fields in
 * unpadded base64url (webauthn-rs), and expects the browser's answer in the same form.
 * `PublicKeyCredential.parseCreationOptionsFromJSON` does this in new browsers only, so it is
 * done here by hand.
 */

type Json = Record<string, unknown>

/** Whether this browser can make and use passkeys at all. */
export function passkeysSupported(): boolean {
  return typeof window !== 'undefined' && typeof window.PublicKeyCredential === 'function'
}

export function base64urlToBuffer(s: string): ArrayBuffer {
  const b64 = s.replace(/-/g, '+').replace(/_/g, '/')
  const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4)
  const bin = atob(padded)
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out.buffer
}

export function bufferToBase64url(buf: ArrayBuffer | ArrayBufferView): string {
  const bytes =
    buf instanceof ArrayBuffer ? new Uint8Array(buf) : new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength)
  let bin = ''
  for (const b of bytes) bin += String.fromCharCode(b)
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function withIds(list: unknown): PublicKeyCredentialDescriptor[] | undefined {
  if (!Array.isArray(list)) return undefined
  return list.map(
    (c: Json) => ({ ...(c as object), id: base64urlToBuffer(String(c.id)) }) as PublicKeyCredentialDescriptor
  )
}

/** `navigator.credentials.create` options from the API's `{ publicKey: … }`. */
export function toCreationOptions(options: Json): CredentialCreationOptions {
  const pk = options.publicKey as Json
  const user = pk.user as Json
  return {
    publicKey: {
      ...(pk as object),
      challenge: base64urlToBuffer(String(pk.challenge)),
      user: { ...(user as object), id: base64urlToBuffer(String(user.id)) },
      excludeCredentials: withIds(pk.excludeCredentials),
    } as PublicKeyCredentialCreationOptions,
  }
}

/** `navigator.credentials.get` options from the API's `{ publicKey: … }`. */
export function toRequestOptions(options: Json): CredentialRequestOptions {
  const pk = options.publicKey as Json
  return {
    publicKey: {
      ...(pk as object),
      challenge: base64urlToBuffer(String(pk.challenge)),
      allowCredentials: withIds(pk.allowCredentials),
    } as PublicKeyCredentialRequestOptions,
  }
}

/** A new passkey as the API reads it. */
export function registrationToJSON(cred: PublicKeyCredential): Json {
  const res = cred.response as AuthenticatorAttestationResponse
  return {
    id: cred.id,
    rawId: bufferToBase64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: bufferToBase64url(res.attestationObject),
      clientDataJSON: bufferToBase64url(res.clientDataJSON),
      transports: typeof res.getTransports === 'function' ? res.getTransports() : undefined,
    },
    extensions: cred.getClientExtensionResults(),
  }
}

/** A passkey's sign-in answer as the API reads it. */
export function assertionToJSON(cred: PublicKeyCredential): Json {
  const res = cred.response as AuthenticatorAssertionResponse
  return {
    id: cred.id,
    rawId: bufferToBase64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: bufferToBase64url(res.authenticatorData),
      clientDataJSON: bufferToBase64url(res.clientDataJSON),
      signature: bufferToBase64url(res.signature),
      userHandle: res.userHandle ? bufferToBase64url(res.userHandle) : undefined,
    },
    extensions: cred.getClientExtensionResults(),
  }
}

/** True when the person closed the browser's passkey dialog or it timed out: not worth an error. */
export function isPasskeyCancel(e: unknown): boolean {
  return e instanceof DOMException && (e.name === 'NotAllowedError' || e.name === 'AbortError')
}

/** A name to start from, so most people can just press Add. */
export function suggestedPasskeyName(ua: string = typeof navigator === 'undefined' ? '' : navigator.userAgent): string {
  if (/iPhone/.test(ua)) return 'iPhone'
  if (/iPad/.test(ua)) return 'iPad'
  if (/Android/.test(ua)) return 'Android'
  if (/Mac OS X|Macintosh/.test(ua)) return 'Mac'
  if (/Windows/.test(ua)) return 'Windows'
  if (/Linux/.test(ua)) return 'Linux'
  return 'This device'
}
