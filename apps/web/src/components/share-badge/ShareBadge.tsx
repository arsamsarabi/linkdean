import type { ReactElement } from 'react'
import { Badge, Tooltip } from '@mantine/core'
import { IconShare } from '@tabler/icons-react'
import type { InviteeShareAccess } from '@/lib'

type Props = {
  access: InviteeShareAccess
  size?: 'xs' | 'sm'
}

export function ShareBadge({ access, size = 'xs' }: Props): ReactElement {
  const ro = access === 'read'
  const label = ro ? 'Shared · RO' : 'Shared · RW'
  const tip = ro ? 'Shared with you (read-only)' : 'Shared with you (can edit)'

  return (
    <Tooltip label={tip}>
      <Badge
        size={size}
        variant={ro ? 'outline' : 'light'}
        color="blue"
        leftSection={<IconShare size={10} stroke={1.5} />}
      >
        {label}
      </Badge>
    </Tooltip>
  )
}
