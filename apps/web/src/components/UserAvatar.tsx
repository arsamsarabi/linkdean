import { useEffect, useState } from 'react'
import { Avatar } from '@mantine/core'
import { getToken } from '../api'

type Props = {
  hasAvatar: boolean
  label: string
  size?: number | string
  radius?: string | number
  bust?: number | string
  color?: string
  alt?: string
}

export function UserAvatar({
  hasAvatar,
  label,
  size,
  radius = 'xl',
  bust,
  color = 'blue',
  alt,
}: Props) {
  const [src, setSrc] = useState<string | undefined>()

  useEffect(() => {
    let objectUrl: string | undefined
    let cancelled = false
    if (!hasAvatar) {
      setSrc((prev) => (prev === undefined ? prev : undefined))
      return
    }
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

  const initials = label
    .split(/[\s@]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((s) => s[0]?.toUpperCase() ?? '')
    .join('')

  return (
    <Avatar src={src} size={size} radius={radius} color={color} alt={alt || label}>
      {initials}
    </Avatar>
  )
}
