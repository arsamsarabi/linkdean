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
  UnstyledButton,
} from '@mantine/core'
import { IconBell, IconCheck } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  useMarkAllNotificationsRead,
  useMarkNotificationRead,
  useNotifications,
  useUnreadNotificationCount,
} from '@/hooks'
import type { Notification } from '@/schemas'

type Props = {
  opened: boolean
  onClose: () => void
}

function hrefFor(n: Notification): string {
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

function kindLabel(kind: Notification['kind']): string {
  switch (kind) {
    case 'share_invite':
      return 'Invite'
    case 'share_access_changed':
      return 'Access'
    case 'share_revoked':
      return 'Revoked'
    case 'shared_edit':
      return 'Edit'
    case 'social_like':
      return 'Like'
    case 'social_comment':
      return 'Comment'
    case 'social_rating':
      return 'Rating'
  }
}

export function NotificationsDrawer({ opened, onClose }: Props): ReactElement {
  const nav = useNavigate()
  const list = useNotifications(opened)
  const unread = useUnreadNotificationCount(true)
  const markOne = useMarkNotificationRead()
  const markAll = useMarkAllNotificationsRead()
  const prevOpened = useRef(opened)
  const unreadCount = unread.data?.count ?? 0
  const hasUnread =
    unreadCount > 0 || (list.data?.notifications.some((n) => !n.read_at) ?? false)

  useEffect(() => {
    const wasOpen = prevOpened.current
    prevOpened.current = opened
    if (wasOpen === opened) return
    // Open or close → mark everything read (idempotent if already clear).
    void markAll.mutateAsync().catch(() => undefined)
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- opened edges only
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
    <Drawer opened={opened} onClose={onClose} position="right" size="md" title="Notifications" padding="md">
      <Stack gap="md">
        <Group justify="space-between">
          <Text size="sm" c="dimmed">
            Shared activity
          </Text>
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

        {list.isError ? (
          <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} />
        ) : null}

        {list.isLoading ? (
          <Stack gap="sm">
            <Skeleton height={56} />
            <Skeleton height={56} />
            <Skeleton height={56} />
          </Stack>
        ) : null}

        {!list.isLoading && (list.data?.notifications.length ?? 0) === 0 ? (
          <Text size="sm" c="dimmed">
            No notifications yet.
          </Text>
        ) : null}

        <Stack gap="xs">
          {(list.data?.notifications ?? []).map((n) => {
            const read = !!n.read_at
            return (
              <UnstyledButton
                key={n.id}
                onClick={() => void openItem(n)}
                p="sm"
                style={{
                  borderRadius: 8,
                  border: '1px solid var(--mantine-color-default-border)',
                  borderLeft: read
                    ? '4px solid var(--mantine-color-gray-5)'
                    : '4px solid var(--mantine-color-violet-6)',
                  background: read ? undefined : 'var(--mantine-color-violet-light)',
                  opacity: read ? 0.72 : 1,
                }}
              >
                <Stack gap={4}>
                  <Group gap="xs" justify="space-between" wrap="nowrap">
                    <Badge size="xs" variant="light" color={read ? 'gray' : 'violet'}>
                      {kindLabel(n.kind)}
                    </Badge>
                    <Text size="xs" c="dimmed">
                      {new Date(n.created_at).toLocaleString()}
                    </Text>
                  </Group>
                  <Text size="sm" fw={read ? 400 : 560} c={read ? 'dimmed' : undefined}>
                    {n.body}
                  </Text>
                </Stack>
              </UnstyledButton>
            )
          })}
        </Stack>
      </Stack>
    </Drawer>
  )
}

export function NotificationsBell({
  unread,
  onClick,
}: {
  unread: number
  onClick: () => void
}): ReactElement {
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
