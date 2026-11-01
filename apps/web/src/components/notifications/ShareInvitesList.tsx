import type { ReactElement } from 'react'
import { Button, Card, Group, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useAcceptShareInvite, useRejectShareInvite, useShareInvites } from '@/hooks'
import type { ShareInvite } from '@/schemas'

function describeInvite(invite: ShareInvite): string {
  const kind = invite.subject_type === 'dashboard' ? 'dashboard' : 'folder'
  const access = invite.access === 'write' ? 'read-write' : 'read-only'
  return `${invite.inviter_display_name} invited you to ${kind} “${invite.subject_title}” (${access})`
}

export function ShareInvitesList(): ReactElement | null {
  const list = useShareInvites()
  const accept = useAcceptShareInvite()
  const reject = useRejectShareInvite()
  const pending = (list.data?.invites ?? []).filter((i) => !i.rejected_at)

  if (pending.length === 0) return null

  async function run(action: Promise<unknown>, message: string): Promise<void> {
    try {
      await action
      notifications.show({ message })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Stack gap="xs">
      {pending.map((invite) => (
        <Card key={invite.id} withBorder padding="sm" radius="md">
          <Stack gap="xs">
            <Text size="sm">{describeInvite(invite)}</Text>
            <Group gap="xs" justify="flex-end">
              <Button
                size="compact-sm"
                variant="default"
                disabled={reject.isPending || accept.isPending}
                onClick={() => void run(reject.mutateAsync(invite.id), 'Invite declined')}
              >
                Decline
              </Button>
              <Button
                size="compact-sm"
                disabled={reject.isPending || accept.isPending}
                onClick={() => void run(accept.mutateAsync(invite.id), 'Invite accepted')}
              >
                Accept
              </Button>
            </Group>
          </Stack>
        </Card>
      ))}
    </Stack>
  )
}
