import { useEffect, useState } from 'react'
import { getToken } from '@/lib'

type Loaded = { src: string; url?: string }

/**
 * Loads an `/api/` image with the page's own token. An `<img>` request only carries the
 * session cookie, which localhost ports share, so it can belong to another user or be gone.
 * Other URLs pass through unchanged.
 */
export function useApiImage(src: string | null | undefined): { src: string | undefined; failed: boolean } {
  const [loaded, setLoaded] = useState<Loaded | null>(null)
  const isApi = !!src?.startsWith('/api/')

  useEffect(() => {
    if (!src || !isApi) return
    let objectUrl: string | undefined
    let cancelled = false
    const token = getToken()
    fetch(src, {
      credentials: 'include',
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    })
      .then((r) => {
        if (!r.ok) throw new Error('image')
        return r.blob()
      })
      .then((blob) => {
        if (cancelled) return
        objectUrl = URL.createObjectURL(blob)
        setLoaded({ src, url: objectUrl })
      })
      .catch(() => {
        if (!cancelled) setLoaded({ src })
      })
    return () => {
      cancelled = true
      if (objectUrl) URL.revokeObjectURL(objectUrl)
    }
  }, [src, isApi])

  if (!isApi) return { src: src ?? undefined, failed: false }
  const current = loaded?.src === src ? loaded : null
  return { src: current?.url, failed: !!current && !current.url }
}
