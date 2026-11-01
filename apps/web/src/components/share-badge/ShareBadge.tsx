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
  /** Owner-only: invitee labels shown in the tooltip. */
  sharedWith?: readonly string[]
  size?: 'xs' | 'sm'
}

type Props = InviteeProps | OutgoingProps

function outgoingTooltip(sharedWith?: readonly string[]): string {
  const names = (sharedWith || []).map((n) => n.trim()).filter(Boolean)
  if (names.length === 0) return 'Shared with others'
  if (names.length === 1) return `Shared with ${names[0]}`
  if (names.length === 2) return `Shared with ${names[0]} and ${names[1]}`
  const head = names.slice(0, -1).join(', ')
  return `Shared with ${head}, and ${names[names.length - 1]}`
}

export function ShareBadge(props: Props): ReactElement {
  const size = props.size ?? 'xs'

  if (props.outgoing) {
    return (
      <Tooltip label={outgoingTooltip(props.sharedWith)}>
        <Badge size={size} variant="light" color="blue" tt="none" leftSection={<IconShare size={10} stroke={1.5} />}>
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
