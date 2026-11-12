export function hostname(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '')
  } catch {
    return url
  }
}

// Known issue: the API's CSP allows images only from 'self', data: and blob:, so the browser
// blocks this URL, logs a console error per tile and the tile falls back to its placeholder.
// It would also send every bookmarked hostname to Google. Fix: drop the fallback and rely on
// the stored favicon, or serve a fallback through the API.
export function googleFavicon(url: string): string | undefined {
  const host = hostname(url)
  if (!host || host === url) return undefined
  return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(host)}&sz=64`
}
