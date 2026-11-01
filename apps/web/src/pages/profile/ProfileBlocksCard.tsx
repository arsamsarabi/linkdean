import { type ReactElement } from 'react'
import { Button, Card, Group, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { QueryErrorAlert } from '@/components'
import { useMyBlocked, useMyMuted, useUnblockUser, useUnmuteUser } from '@/hooks'
import type { FollowUser } from '@/schemas'

type UserListProps = {
  heading: string
  emptyText: string
  actionLabel: string
  users: FollowUser[]
  isLoading: boolean
  pendingId?: string
  onAction: (id: string) => void
}

function UserList({
  heading,
  emptyText,
  actionLabel,
  users,
  isLoading,
  pendingId,
  onAction,
}: UserListProps): ReactElement {
  return (
    <Stack gap={4}>
      <Text size="sm" fw={500}>
        {heading}
      </Text>
      {isLoading ? (
        <Text size="sm" c="dimmed">
          Loading…
        </Text>
      ) : users.length === 0 ? (
        <Text size="sm" c="dimmed">
          {emptyText}
        </Text>
      ) : (
        users.map((u) => (
          <Group key={u.id} justify="space-between" wrap="nowrap">
            <Text size="sm" truncate>
              {u.display_name}
            </Text>
            <Button
              size="compact-xs"
              variant="default"
              aria-label={`${actionLabel} ${u.display_name}`}
              loading={pendingId === u.id}
              onClick={() => onAction(u.id)}
            >
              {actionLabel}
            </Button>
          </Group>
        ))
      )}
    </Stack>
  )
}

export function ProfileBlocksCard(): ReactElement {
  const blocked = useMyBlocked()
  const muted = useMyMuted()
  const unblock = useUnblockUser()
  const unmute = useUnmuteUser()

  async function onUnblock(id: string): Promise<void> {
    try {
      await unblock.mutateAsync(id)
      notifications.show({ message: 'Unblocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onUnmute(id: string): Promise<void> {
    try {
      await unmute.mutateAsync(id)
      notifications.show({ message: 'Unmuted' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Card withBorder padding="lg" radius="md">
      <Stack gap="md">
        <Title order={4}>Blocked and muted</Title>

        {blocked.isError ? (
          <QueryErrorAlert error={blocked.error} onRetry={() => void blocked.refetch()} />
        ) : (
          <UserList
            heading="Blocked"
            emptyText="Nobody blocked — use ⋮ on a shared bookmark to block its owner."
            actionLabel="Unblock"
            users={blocked.data?.users ?? []}
            isLoading={blocked.isLoading}
            pendingId={unblock.isPending ? unblock.variables : undefined}
            onAction={(id) => void onUnblock(id)}
          />
        )}

        {muted.isError ? (
          <QueryErrorAlert error={muted.error} onRetry={() => void muted.refetch()} />
        ) : (
          <UserList
            heading="Muted"
            emptyText="Nobody muted."
            actionLabel="Unmute"
            users={muted.data?.users ?? []}
            isLoading={muted.isLoading}
            pendingId={unmute.isPending ? unmute.variables : undefined}
            onAction={(id) => void onUnmute(id)}
          />
        )}
      </Stack>
    </Card>
  )
}
