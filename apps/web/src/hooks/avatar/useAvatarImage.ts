import { useEffect, useState } from 'react'
import { getToken } from '@/lib'
import { useSyncDuringRender } from '../render'

export function useAvatarImage(hasAvatar: boolean, bust?: number | string): string | undefined {
  const [src, setSrc] = useState<string | undefined>()

  useSyncDuringRender([hasAvatar], () => {
    if (!hasAvatar) setSrc(undefined)
  })

  useEffect(() => {
    let objectUrl: string | undefined
    let cancelled = false
    if (!hasAvatar) return
    const token = getToken()
    const q = bust != null ? `?v=${bust}` : ''
    fetch(`/api/users/me/avatar${q}`, {
      credentials: 'include',
      headers: token ? { Authorization: `Bearer ${token}` } : {},
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
  }, [hasAvatar, bust])

  return src
}
