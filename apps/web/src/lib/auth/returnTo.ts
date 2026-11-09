/** Where a signed-out visitor was headed, kept for this tab while they sign in. */
const returnToKey = 'linkdean_return_to'

/**
 * Only quick save comes back after sign-in: it carries the page being saved, which would
 * otherwise be lost. Every other page lands where sign-in always lands.
 */
function isReturnPath(path: string): boolean {
  return path === '/save' || path.startsWith('/save?')
}

export function rememberReturnTo(path: string): void {
  if (!isReturnPath(path)) return
  try {
    sessionStorage.setItem(returnToKey, path)
  } catch {
    // Storage blocked: sign-in lands on the usual page.
  }
}

/** The remembered path, once: reading it clears it. */
export function takeReturnTo(): string | null {
  try {
    const path = sessionStorage.getItem(returnToKey)
    sessionStorage.removeItem(returnToKey)
    return path && isReturnPath(path) ? path : null
  } catch {
    return null
  }
}
