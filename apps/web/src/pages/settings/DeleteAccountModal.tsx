import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, List, Modal, PasswordInput, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconTrash } from '@tabler/icons-react'
import { ApiError } from '@/api'
import { useScheduleAccountDeletion } from '@/hooks'

type Props = {
  email: string
  hasPassword: boolean
  onClose: () => void
}

function errorMessage(e: Error): string {
  if (e instanceof ApiError && e.status === 401) return 'Wrong password.'
  return e.message
}

export function formatDeletionDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { dateStyle: 'long' })
}

export function DeleteAccountModal({ email, hasPassword, onClose }: Props): ReactElement {
  const schedule = useScheduleAccountDeletion()
  const [confirmEmail, setConfirmEmail] = useState('')
  const [password, setPassword] = useState('')

  const emailOk = confirmEmail.trim().toLowerCase() === email.toLowerCase()
  const ready = emailOk && (!hasPassword || !!password)

  function submit(): void {
    if (schedule.isPending || !ready) return
    schedule.mutate(
      { confirm_email: confirmEmail.trim(), ...(hasPassword ? { password } : {}) },
      {
        onSuccess: (res) => {
          notifications.show({ message: `Account will be deleted on ${formatDeletionDate(res.deletion_scheduled_at)}` })
          onClose()
        },
      }
    )
  }

  return (
    <Modal opened onClose={onClose} title="Delete account">
      <Stack>
        <Text size="sm">
          Your account is deleted after a 7-day grace period. Until then you can sign in and cancel from Config options.
        </Text>
        <List size="sm" c="dimmed" spacing={4}>
          <List.Item>Bookmarks, categories, vaults, files, API keys and sessions are erased.</List.Item>
          <List.Item>Shared folders you own are deleted. Everyone you shared them with loses access.</List.Item>
          <List.Item>Export your data first (Import / Export) if you want a copy.</List.Item>
        </List>
        {schedule.isError ? (
          <Alert color="red" variant="light">
            {errorMessage(schedule.error)}
          </Alert>
        ) : null}
        <TextInput
          label={`Type ${email} to confirm`}
          autoComplete="off"
          data-autofocus
          value={confirmEmail}
          onChange={(e) => setConfirmEmail(e.currentTarget.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
        {hasPassword ? (
          <PasswordInput
            label="Current password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
        ) : null}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Keep account
          </Button>
          <Button
            color="red"
            leftSection={<IconTrash size={16} stroke={1.5} />}
            loading={schedule.isPending}
            disabled={!ready}
            onClick={submit}
          >
            Delete account
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
