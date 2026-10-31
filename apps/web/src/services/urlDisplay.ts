export function hostname(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '')
  } catch {
    return url
  }
}

export function googleFavicon(url: string): string | undefined {
  const host = hostname(url)
  if (!host || host === url) return undefined
  return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(host)}&sz=64`
}
