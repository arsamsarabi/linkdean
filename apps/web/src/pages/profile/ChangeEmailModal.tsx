import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Modal, PasswordInput, Stack, Text, TextInput } from '@mantine/core'
import { IconSend } from '@tabler/icons-react'
import { useChangeEmail } from '@/hooks'

type Props = {
  currentEmail: string
  onClose: () => void
}

/** Sends a confirmation link to the new address. The email stays the same until it is opened. */
export function ChangeEmailModal({ currentEmail, onClose }: Props): ReactElement {
  const change = useChangeEmail()
  const [newEmail, setNewEmail] = useState('')
  const [password, setPassword] = useState('')
  const sentTo = change.variables?.new_email

  function submit(): void {
    if (change.isPending || !newEmail.trim() || !password) return
    change.mutate({ password, new_email: newEmail.trim() })
  }

  return (
    <Modal opened onClose={onClose} title="Change email">
      {change.isSuccess && sentTo ? (
        <Stack>
          <Text size="sm">
            Check <b>{sentTo}</b> for a link. Your email stays the same until you open it. The link works for{' '}
            {change.data.hours} hours.
          </Text>
          <Group justify="flex-end">
            <Button onClick={onClose}>Done</Button>
          </Group>
        </Stack>
      ) : (
        <Stack>
          <Text size="sm" c="dimmed">
            Your email is {currentEmail}. We send a link to the new address to confirm it.
          </Text>
          {change.isError ? (
            <Alert color="red" variant="light">
              {change.error.message}
            </Alert>
          ) : null}
          <TextInput
            label="New email"
            type="email"
            autoComplete="email"
            data-autofocus
            value={newEmail}
            onChange={(e) => setNewEmail(e.currentTarget.value)}
          />
          <PasswordInput
            label="Current password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
          <Group justify="flex-end">
            <Button variant="default" onClick={onClose}>
              Cancel
            </Button>
            <Button
              leftSection={<IconSend size={16} stroke={1.5} />}
              loading={change.isPending}
              disabled={!newEmail.trim() || !password}
              onClick={submit}
            >
              Send confirmation link
            </Button>
          </Group>
        </Stack>
      )}
    </Modal>
  )
}
