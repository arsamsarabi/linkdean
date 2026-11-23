import type { ReactElement } from 'react'
import { Button, Card, Group, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useAcceptOrganizationInvite, useOrganizationInviteInbox, useRejectOrganizationInvite } from '@/hooks'

export function OrganizationInvitesList(): ReactElement | null {
  const list = useOrganizationInviteInbox()
  const accept = useAcceptOrganizationInvite()
  const reject = useRejectOrganizationInvite()
  const pending = list.data ?? []

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
            <Text size="sm">
              {invite.inviter_display_name} invited you to join “{invite.organization_name}” as {invite.role}
            </Text>
            <Group gap="xs" justify="flex-end">
              <Button
                size="compact-sm"
                variant="default"
                disabled={reject.isPending || accept.isPending}
                onClick={() => void run(reject.mutateAsync(invite.id), 'Organization invite declined')}
              >
                Decline
              </Button>
              <Button
                size="compact-sm"
                disabled={reject.isPending || accept.isPending}
                onClick={() => void run(accept.mutateAsync(invite.id), 'Joined organization')}
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
