import { type ReactElement } from 'react'
import { Badge, Group, Tooltip } from '@mantine/core'
import { IconBookmarkFilled, IconLinkOff, IconLock, IconPinFilled, IconStarFilled } from '@tabler/icons-react'
import { ShareBadge } from '@/components/share-badge'
import type { BookmarkCardItem } from './bookmarkCardTypes'

type Props = {
  bookmark: BookmarkCardItem
  share: 'read' | 'write' | null
  showOutgoing: boolean
  pendingDelete: boolean
}

export function BookmarkCardBadges({ bookmark: b, share, showOutgoing, pendingDelete }: Props): ReactElement {
  return (
    <Group gap={4} mt="auto" pt={4}>
      {share ? <ShareBadge access={share} ownerLabel={b.owner_display_name} size="sm" /> : null}
      {showOutgoing ? <ShareBadge outgoing sharedWith={b.shared_with} size="sm" /> : null}
      {pendingDelete ? (
        <Badge size="sm" variant="outline" color="red" tt="none">
          Pending delete
        </Badge>
      ) : null}
      {b.link_check_status === 'broken' || b.link_check_status === 'unreachable' ? (
        <Tooltip
          label={
            b.link_check_status_code != null
              ? `${b.link_check_status} · HTTP ${b.link_check_status_code}`
              : b.link_check_error || b.link_check_status
          }
        >
          <Badge size="sm" variant="light" color="red" leftSection={<IconLinkOff size={10} stroke={1.5} />}>
            {b.link_check_status}
          </Badge>
        </Tooltip>
      ) : null}
      {b.is_favorite ? (
        <Badge size="sm" variant="light" color="yellow" leftSection={<IconStarFilled size={10} stroke={1.5} />}>
          favorite
        </Badge>
      ) : null}
      {b.is_unread ? (
        <Badge size="sm" variant="light" color="blue" leftSection={<IconBookmarkFilled size={10} stroke={1.5} />}>
          unread
        </Badge>
      ) : null}
      {b.is_pinned ? (
        <Badge size="sm" variant="light" leftSection={<IconPinFilled size={10} stroke={1.5} />}>
          pinned
        </Badge>
      ) : null}
      {b.is_vault ? (
        <Tooltip label="Stored in vault">
          <Badge size="sm" leftSection={<IconLock size={10} stroke={1.5} />}>
            vault
          </Badge>
        </Tooltip>
      ) : null}
      {(b.tags || []).map((t) => (
        <Badge key={t} variant="outline" size="sm">
          {t}
        </Badge>
      ))}
    </Group>
  )
}
