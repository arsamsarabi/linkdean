/** Where a signed-out visitor was headed, kept for this tab while they sign in. */
const returnToKey = 'linkdean_return_to'

/**
 * Only quick save and the extension's sign-in come back after sign-in: they carry what the
 * person was in the middle of, which would otherwise be lost. Every other page lands where
 * sign-in always lands.
 */
function isReturnPath(path: string): boolean {
  return ['/save', '/connect/extension'].some((p) => path === p || path.startsWith(`${p}?`))
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
