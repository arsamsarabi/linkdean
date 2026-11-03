/**
 * The browser's session lives only in the API's `HttpOnly` cookie, out of reach of page scripts.
 * This key holds a non-secret marker that this browser signed in with a password, so routes can
 * skip a round trip and other tabs can follow sign-in and sign-out.
 */
export const sessionMarkerKey = 'linkdean_signed_in'

/** Where older versions kept the session token itself. Removed on load. */
const legacyTokenKey = 'linkdean_token'

/** Header the API requires on cookie-authenticated writes; cross-site pages cannot send it. */
export const CLIENT_HEADER = 'X-Linkdean-Client'

export function isSignedIn(): boolean {
  try {
    return localStorage.getItem(sessionMarkerKey) === '1'
  } catch {
    return false
  }
}

export function markSignedIn(signedIn: boolean): void {
  try {
    if (signedIn) localStorage.setItem(sessionMarkerKey, '1')
    else localStorage.removeItem(sessionMarkerKey)
  } catch {
    // Storage blocked: the cookie still carries the session.
  }
}

/** Swap a token left by an older version for the marker; the cookie set with it still works. */
export function dropLegacyToken(): void {
  try {
    if (localStorage.getItem(legacyTokenKey) === null) return
    localStorage.removeItem(legacyTokenKey)
    markSignedIn(true)
  } catch {
    // Nothing to migrate.
  }
}
