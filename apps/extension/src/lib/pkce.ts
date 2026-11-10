function base64url(bytes: Uint8Array): string {
  let s = ''
  for (const b of bytes) s += String.fromCharCode(b)
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

/** A random PKCE verifier: 32 bytes, 43 base64url characters. */
export function randomVerifier(): string {
  return base64url(crypto.getRandomValues(new Uint8Array(32)))
}

/** The `S256` challenge for `verifier`. */
export async function challengeFor(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier))
  return base64url(new Uint8Array(digest))
}

/** A random value to match the sign-in reply with the request. */
export function randomState(): string {
  return base64url(crypto.getRandomValues(new Uint8Array(16)))
}
