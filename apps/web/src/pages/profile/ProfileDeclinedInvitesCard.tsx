import type { ReactElement } from 'react'
import { Button, Card, Group, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useRestoreShareInvite, useShareInvites } from '@/hooks'

export function ProfileDeclinedInvitesCard(): ReactElement | null {
  const list = useShareInvites()
  const restore = useRestoreShareInvite()
  const declined = (list.data?.invites ?? []).filter((i) => !!i.rejected_at)

  if (declined.length === 0) return null

  async function onRestore(id: string): Promise<void> {
    try {
      await restore.mutateAsync(id)
      notifications.show({ message: 'Invite restored to your notifications' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Card withBorder padding="lg" radius="md">
      <Stack gap="md">
        <Title order={4}>Declined invites</Title>
        <Text size="sm" c="dimmed">
          People can't re-invite you to these until you restore them.
        </Text>
        {declined.map((invite) => (
          <Group key={invite.id} justify="space-between" wrap="nowrap">
            <Text size="sm">
              {invite.subject_title} from {invite.inviter_display_name}
            </Text>
            <Button
              size="compact-sm"
              variant="default"
              loading={restore.isPending}
              onClick={() => void onRestore(invite.id)}
            >
              Restore
            </Button>
          </Group>
        ))}
      </Stack>
    </Card>
  )
}
