import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Modal, PasswordInput, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLinkOff } from '@tabler/icons-react'
import { ApiError } from '@/api'
import { useDisconnectSso } from '@/hooks'

type Props = {
  onClose: () => void
}

function errorMessage(e: Error): string {
  // A wrong password is a plain 401; the session is still good.
  if (e instanceof ApiError && e.status === 401) return 'Wrong password.'
  return e.message
}

export function DisconnectSsoModal({ onClose }: Props): ReactElement {
  const disconnect = useDisconnectSso()
  const [password, setPassword] = useState('')

  function submit(): void {
    if (disconnect.isPending || !password) return
    disconnect.mutate(password, {
      onSuccess: () => {
        notifications.show({ message: 'Single sign-on disconnected' })
        onClose()
      },
    })
  }

  return (
    <Modal opened onClose={onClose} title="Disconnect single sign-on">
      <Stack>
        <Text size="sm" c="dimmed">
          You can then sign in only with your email and password.
        </Text>
        {disconnect.isError ? (
          <Alert color="red" variant="light">
            {errorMessage(disconnect.error)}
          </Alert>
        ) : null}
        <PasswordInput
          label="Current password"
          autoComplete="current-password"
          data-autofocus
          value={password}
          onChange={(e) => setPassword(e.currentTarget.value)}
          onKeyDown={(e) => e.key === 'Enter' && submit()}
        />
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button
            color="red"
            leftSection={<IconLinkOff size={16} stroke={1.5} />}
            loading={disconnect.isPending}
            disabled={!password}
            onClick={submit}
          >
            Disconnect
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
