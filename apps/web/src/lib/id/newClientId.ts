/** Client-side id for draft keys. Works outside secure contexts (LAN HTTP). */
export function newClientId(): string {
  try {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID()
    }
  } catch {
    // Not a secure context (e.g. http://192.168.x.x) — fall through.
  }
  return `local-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}
