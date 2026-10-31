import type { ReactElement } from 'react'
import { Badge, Tooltip } from '@mantine/core'
import { IconShare } from '@tabler/icons-react'
import type { InviteeShareAccess } from '@/lib'

type InviteeProps = {
  access: InviteeShareAccess
  outgoing?: false
  /** Owner label for invitee badges (display name / email local-part). */
  ownerLabel?: string | null
  size?: 'xs' | 'sm'
}

type OutgoingProps = {
  outgoing: true
  access?: never
  ownerLabel?: never
  size?: 'xs' | 'sm'
}

type Props = InviteeProps | OutgoingProps

export function ShareBadge(props: Props): ReactElement {
  const size = props.size ?? 'xs'

  if (props.outgoing) {
    return (
      <Tooltip label="Shared with others">
        <Badge
          size={size}
          variant="light"
          color="blue"
          tt="none"
          leftSection={<IconShare size={10} stroke={1.5} />}
        >
          Shared
        </Badge>
      </Tooltip>
    )
  }

  const ro = props.access === 'read'
  const owner = props.ownerLabel?.trim() || 'Owner'
  const label = ro ? `${owner} · RO` : `${owner} · RW`
  const tip = ro ? `Shared by ${owner} (read-only)` : `Shared by ${owner} (can edit)`

  return (
    <Tooltip label={tip}>
      <Badge
        size={size}
        variant={ro ? 'outline' : 'light'}
        color="blue"
        tt="none"
        leftSection={<IconShare size={10} stroke={1.5} />}
      >
        {label}
      </Badge>
    </Tooltip>
  )
}
