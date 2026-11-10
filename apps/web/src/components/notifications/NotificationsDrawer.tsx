import { useEffect, useRef, type ReactElement } from 'react'
import {
  ActionIcon,
  Badge,
  Button,
  Drawer,
  Group,
  Skeleton,
  Stack,
  Text,
  ThemeIcon,
  UnstyledButton,
} from '@mantine/core'
import {
  IconBell,
  IconBellOff,
  IconCheck,
  IconHeart,
  IconKey,
  IconLinkOff,
  IconMailShare,
  IconMessageCircle,
  IconPencil,
  IconShieldExclamation,
  IconStar,
  IconUserPlus,
} from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  useMarkAllNotificationsRead,
  useMarkNotificationRead,
  useNotifications,
  useShareInvites,
  useUnreadNotificationCount,
} from '@/hooks'
import { useDrawerSize } from '@/hooks/render'
import type { Notification, ShareInvite } from '@/schemas'
import styles from './NotificationsDrawer.module.css'
import { ShareInvitesList } from './ShareInvitesList'

type Props = {
  opened: boolean
  onClose: () => void
}

/** An unanswered invite already shows as a card with its actions, so its row would repeat it. */
function withoutOpenInvites(rows: Notification[], invites: ShareInvite[]): Notification[] {
  const open = new Set(invites.filter((i) => !i.rejected_at).map((i) => `${i.subject_type}:${i.subject_id}`))
  return rows.filter((n) => n.kind !== 'share_invite' || !open.has(`${n.subject_type}:${n.subject_id}`))
}

function hrefFor(n: Notification): string {
  if (n.kind === 'moderation_changes') return '/trash'
  switch (n.subject_type) {
    case 'dashboard':
      return '/'
    case 'category':
    case 'bookmark':
      return '/library'
    default:
      return '/'
  }
}

const KINDS: Record<Notification['kind'], { label: string; icon: typeof IconBell; color: string }> = {
  share_invite: { label: 'Invite', icon: IconMailShare, color: 'violet' },
  share_access_changed: { label: 'Access changed', icon: IconKey, color: 'blue' },
  share_revoked: { label: 'Access removed', icon: IconLinkOff, color: 'gray' },
  shared_edit: { label: 'Edit', icon: IconPencil, color: 'teal' },
  social_like: { label: 'Like', icon: IconHeart, color: 'pink' },
  social_comment: { label: 'Comment', icon: IconMessageCircle, color: 'cyan' },
  social_rating: { label: 'Rating', icon: IconStar, color: 'yellow' },
  user_followed: { label: 'New follower', icon: IconUserPlus, color: 'grape' },
  moderation_changes: { label: 'Moderation', icon: IconShieldExclamation, color: 'red' },
}

function dayLabel(iso: string, now: Date): string {
  const d = new Date(iso)
  const days = Math.round(
    (new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() -
      new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()) /
      86_400_000
  )
  if (days === 0) return 'Today'
  if (days === 1) return 'Yesterday'
  return d.toLocaleDateString(undefined, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric',
  })
}

/** Rows arrive newest first, so consecutive runs of one day form a group. */
function groupByDay(rows: Notification[], now = new Date()): { day: string; rows: Notification[] }[] {
  const groups: { day: string; rows: Notification[] }[] = []
  for (const n of rows) {
    const day = dayLabel(n.created_at, now)
    const last = groups.at(-1)
    if (last?.day === day) last.rows.push(n)
    else groups.push({ day, rows: [n] })
  }
  return groups
}

export function NotificationsDrawer({ opened, onClose }: Props): ReactElement {
  const drawerSize = useDrawerSize()
  const nav = useNavigate()
  const list = useNotifications(opened)
  const unread = useUnreadNotificationCount(true)
  const invites = useShareInvites(opened)
  const markOne = useMarkNotificationRead()
  const markAll = useMarkAllNotificationsRead()
  const prevOpened = useRef(opened)
  const unreadCount = unread.data?.count ?? 0
  const inviteList = invites.data?.invites ?? []
  const rows = withoutOpenInvites(list.data?.notifications ?? [], inviteList)
  const hasPendingInvite = inviteList.some((i) => !i.rejected_at)
  const hasUnread = unreadCount > 0 || (list.data?.notifications.some((n) => !n.read_at) ?? false)

  useEffect(() => {
    const wasOpen = prevOpened.current
    prevOpened.current = opened
    if (!wasOpen || opened) return
    // Unread items keep their highlight while the drawer is open.
    void markAll.mutateAsync().catch(() => undefined)
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- closing edge only
  }, [opened])

  async function openItem(n: Notification): Promise<void> {
    if (!n.read_at) {
      try {
        await markOne.mutateAsync(n.id)
      } catch {
        // still navigate
      }
    }
    onClose()
    void nav({ to: hrefFor(n) })
  }

  return (
    <Drawer opened={opened} onClose={onClose} position="right" size={drawerSize} title="Notifications" padding="md">
      <Stack gap="md">
        <Group justify="space-between">
          {unreadCount > 0 ? (
            <Badge variant="light" color="violet" tt="none">
              {unreadCount} unread
            </Badge>
          ) : (
            <Text size="sm" c="dimmed">
              Shares, comments and follows
            </Text>
          )}
          <Button
            size="compact-xs"
            variant="subtle"
            leftSection={<IconCheck size={14} stroke={1.5} />}
            loading={markAll.isPending}
            disabled={!hasUnread || markAll.isPending}
            onClick={() => void markAll.mutateAsync()}
          >
            Mark all read
          </Button>
        </Group>

        <ShareInvitesList />

        {list.isError ? <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} /> : null}

        {list.isLoading ? (
          <Stack gap="sm">
            <Skeleton height={56} />
            <Skeleton height={56} />
            <Skeleton height={56} />
          </Stack>
        ) : null}

        {!list.isLoading && !list.isError && rows.length === 0 && !hasPendingInvite ? (
          <Stack align="center" gap="xs" py="xl">
            <ThemeIcon variant="light" color="gray" size={56} radius="xl">
              <IconBellOff size={28} stroke={1.5} />
            </ThemeIcon>
            <Text fw={600}>You're all caught up</Text>
            <Text size="sm" c="dimmed" ta="center" maw={280}>
              Invites, comments, likes and new followers will show up here.
            </Text>
          </Stack>
        ) : null}

        {groupByDay(rows).map((group) => (
          <Stack key={group.day} gap={4} component="section" aria-label={group.day}>
            <Text size="xs" fw={600} c="dimmed" tt="uppercase" className={styles.day}>
              {group.day}
            </Text>
            {group.rows.map((n) => {
              const read = !!n.read_at
              const kind = KINDS[n.kind]
              const Icon = kind.icon
              return (
                <UnstyledButton
                  key={n.id}
                  onClick={() => void openItem(n)}
                  className={styles.row}
                  data-unread={read ? undefined : true}
                >
                  <ThemeIcon variant="light" color={kind.color} size={36} radius="xl" aria-hidden>
                    <Icon size={18} stroke={1.5} />
                  </ThemeIcon>
                  <div className={styles.body}>
                    <Text size="sm" fw={read ? 400 : 600} c={read ? 'dimmed' : undefined}>
                      {n.body}
                    </Text>
                    <Text size="xs" c="dimmed">
                      {kind.label} ·{' '}
                      {new Date(n.created_at).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}
                    </Text>
                  </div>
                  {read ? null : <span className={styles.dot} aria-label="Unread" />}
                </UnstyledButton>
              )
            })}
          </Stack>
        ))}
      </Stack>
    </Drawer>
  )
}

export function NotificationsBell({ unread, onClick }: { unread: number; onClick: () => void }): ReactElement {
  return (
    <ActionIcon
      variant="subtle"
      size="lg"
      radius="md"
      aria-label={unread > 0 ? `Notifications, ${unread} unread` : 'Notifications'}
      onClick={onClick}
    >
      <IconBell size={20} stroke={1.5} />
    </ActionIcon>
  )
}
