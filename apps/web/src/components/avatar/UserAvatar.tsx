import type { ReactElement } from 'react'
import { Avatar } from '@mantine/core'
import { useAvatarImage } from '../../hooks/avatar/useAvatarImage'

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
}: Props): ReactElement {
  const src = useAvatarImage(hasAvatar, bust)

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
