import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Paper, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconAlertTriangle, IconTrash } from '@tabler/icons-react'
import { useCancelAccountDeletion } from '@/hooks'
import type { UserPublic } from '@/schemas'
import { DeleteAccountModal, formatDeletionDate } from './DeleteAccountModal'

type Props = {
  user: UserPublic | undefined
}

export function DeleteAccountCard({ user }: Props): ReactElement | null {
  const cancel = useCancelAccountDeletion()
  const [open, setOpen] = useState(false)

  if (!user) return null
  const scheduledAt = user.deletion_scheduled_at

  function onCancel(): void {
    cancel.mutate(undefined, {
      onSuccess: () => notifications.show({ message: 'Account deletion cancelled' }),
      onError: (e) => notifications.show({ color: 'red', message: String(e) }),
    })
  }

  return (
    <Paper p="lg" withBorder style={{ borderColor: 'var(--mantine-color-red-6)' }}>
      <Stack gap="md">
        <Title order={4} c="red">
          Danger zone
        </Title>
        {scheduledAt ? (
          <Alert icon={<IconAlertTriangle size={16} />} color="red" variant="light" title="Deletion scheduled">
            <Group justify="space-between" wrap="nowrap">
              <Text size="sm">
                Your account and everything in it will be deleted on {formatDeletionDate(scheduledAt)}.
              </Text>
              <Button variant="default" size="xs" loading={cancel.isPending} onClick={onCancel}>
                Cancel deletion
              </Button>
            </Group>
          </Alert>
        ) : (
          <Group justify="space-between" wrap="nowrap">
            <Text size="sm" c="dimmed">
              Delete your account after a 7-day grace period. Shared folders you own are deleted too. Export your data
              first.
            </Text>
            <Button
              color="red"
              variant="light"
              leftSection={<IconTrash size={16} stroke={1.5} />}
              onClick={() => setOpen(true)}
            >
              Delete account…
            </Button>
          </Group>
        )}
      </Stack>
      {open ? (
        <DeleteAccountModal email={user.email} hasPassword={!!user.has_password} onClose={() => setOpen(false)} />
      ) : null}
    </Paper>
  )
}
