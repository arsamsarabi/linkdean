import { useEffect, useState } from 'react'
import { useSyncDuringRender } from '../render'

// Known issue: For `me`, the browser can serve the previous account's avatar from HTTP cache for up
// to an hour. See `avatar_response` in the API.
export function useAvatarImage(hasAvatar: boolean, bust?: number | string, userId = 'me'): string | undefined {
  const [src, setSrc] = useState<string | undefined>()

  useSyncDuringRender([hasAvatar], () => {
    if (!hasAvatar) setSrc(undefined)
  })

  useEffect(() => {
    let objectUrl: string | undefined
    let cancelled = false
    if (!hasAvatar) return
    const q = bust != null ? `?v=${bust}` : ''
    fetch(`/api/users/${userId}/avatar${q}`, {
      credentials: 'include',
    })
      .then((r) => {
        if (!r.ok) throw new Error('avatar')
        return r.blob()
      })
      .then((blob) => {
        if (cancelled) return
        objectUrl = URL.createObjectURL(blob)
        setSrc(objectUrl)
      })
      .catch(() => {
        if (!cancelled) setSrc((prev) => (prev === undefined ? prev : undefined))
      })
    return () => {
      cancelled = true
      if (objectUrl) URL.revokeObjectURL(objectUrl)
    }
  }, [hasAvatar, bust, userId])

  return src
}
