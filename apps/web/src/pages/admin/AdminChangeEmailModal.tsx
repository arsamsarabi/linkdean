import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Modal, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useAdminChangeEmail } from '@/hooks'

type Props = {
  user: { id: string; label: string; email: string }
  onClose: () => void
}

/** Changes an account's email at once, with no confirmation link: the admin vouches for the address. */
export function AdminChangeEmailModal({ user, onClose }: Props): ReactElement {
  const change = useAdminChangeEmail()
  const [email, setEmail] = useState(user.email)
  const next = email.trim()

  function submit(): void {
    if (change.isPending || !next || next === user.email) return
    change.mutate(
      { id: user.id, email: next },
      {
        onSuccess: () => {
          notifications.show({ message: `Email changed to ${next}` })
          onClose()
        },
      }
    )
  }

  return (
    <Modal opened onClose={onClose} title={`Change email for ${user.label}`}>
      <Stack>
        <Text size="sm">The new address applies at once and counts as verified. No confirmation link is sent.</Text>
        {change.isError ? (
          <Alert color="red" variant="light">
            {change.error.message}
          </Alert>
        ) : null}
        <TextInput
          label="New email"
          type="email"
          data-autofocus
          value={email}
          onChange={(e) => setEmail(e.currentTarget.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button loading={change.isPending} disabled={!next || next === user.email} onClick={submit}>
            Change email
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
