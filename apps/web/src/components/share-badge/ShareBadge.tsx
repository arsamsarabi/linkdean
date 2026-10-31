import type { ReactElement } from 'react'
import { Badge, Tooltip } from '@mantine/core'
import { IconShare } from '@tabler/icons-react'
import type { InviteeShareAccess } from '@/lib'

type InviteeProps = {
  access: InviteeShareAccess
  outgoing?: false
  size?: 'xs' | 'sm'
}

type OutgoingProps = {
  outgoing: true
  access?: never
  size?: 'xs' | 'sm'
}

type Props = InviteeProps | OutgoingProps

export function ShareBadge(props: Props): ReactElement {
  const size = props.size ?? 'xs'

  if (props.outgoing) {
    return (
      <Tooltip label="Shared with others">
        <Badge size={size} variant="light" color="blue" leftSection={<IconShare size={10} stroke={1.5} />}>
          Shared
        </Badge>
      </Tooltip>
    )
  }

  const ro = props.access === 'read'
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
