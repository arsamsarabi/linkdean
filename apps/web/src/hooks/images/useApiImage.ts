import { useEffect, useState } from 'react'

type Loaded = { src: string; url?: string }

/**
 * Loads an `/api/` image through `fetch` with the session cookie, so a failure shows as
 * `failed` instead of a broken `<img>`. Other URLs pass through unchanged.
 */
export function useApiImage(src: string | null | undefined): { src: string | undefined; failed: boolean } {
  const [loaded, setLoaded] = useState<Loaded | null>(null)
  const isApi = !!src?.startsWith('/api/')

  useEffect(() => {
    if (!src || !isApi) return
    let objectUrl: string | undefined
    let cancelled = false
    fetch(src, {
      credentials: 'include',
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
