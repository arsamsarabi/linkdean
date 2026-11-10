/** The hosted service; self-hosted instances are typed in instead. */
export const DEFAULT_INSTANCE = 'https://app.linkdean.app'

/**
 * The origin of a Linkdean instance from what the person typed: `app.linkdean.app`,
 * `https://links.example.com/library`, and so on. Plain `http://` only for this machine, so a
 * key never crosses a network unencrypted.
 */
export function normalizeInstance(input: string): string {
  const trimmed = input.trim()
  if (!trimmed) throw new Error('Enter the address of your Linkdean, such as app.linkdean.app.')
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`
  let url: URL
  try {
    url = new URL(withScheme)
  } catch {
    throw new Error('That does not look like a web address.')
  }
  const local = url.hostname === 'localhost' || url.hostname === '127.0.0.1'
  if (url.protocol !== 'https:' && !(url.protocol === 'http:' && local)) {
    throw new Error('Use an https:// address. Plain http:// only works for localhost.')
  }
  if (url.username || url.password) throw new Error('Leave the user name and password out of the address.')
  return url.origin
}

/** The match pattern the browser asks the person to allow for `origin`. */
export function originPattern(origin: string): string {
  const url = new URL(origin)
  // Match patterns take no port; the browser then allows any port on that host.
  return `${url.protocol}//${url.hostname}/*`
}
