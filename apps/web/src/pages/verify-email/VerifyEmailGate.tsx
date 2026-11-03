import { useEffect, type ReactElement } from 'react'
import { Alert, Button, Center, Group, Loader, Paper, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCheck, IconMail } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { ApiError } from '@/api'
import { useLogout, useResendVerification, useSessionCheck } from '@/hooks'
import { isGated } from '@/services'

const iconSm = { size: 16, stroke: 1.5 } as const

/** The hard gate: an unverified account can only verify or sign out. Leaves once it may pass. */
export function VerifyEmailGate(): ReactElement {
  const nav = useNavigate()
  const logout = useLogout()
  const check = useSessionCheck()
  const resend = useResendVerification()
  const signedOut = check.error instanceof ApiError && check.error.status === 401
  const gated = check.data ? isGated(check.data) : false

  useEffect(() => {
    if (signedOut) void nav({ to: '/login', replace: true })
    else if (check.data && !gated) void nav({ to: '/', replace: true })
  }, [check.data, gated, nav, signedOut])

  async function signOut(): Promise<void> {
    try {
      await logout.mutateAsync()
    } catch {
      notifications.show({ color: 'red', message: 'Could not sign out. Try again.' })
      return
    }
    await nav({ to: '/login' })
  }

  const signOutButton = (
    <Button variant="default" loading={logout.isPending} onClick={() => void signOut()}>
      Sign out
    </Button>
  )

  if (check.isError && !signedOut) {
    return (
      <Center mih="100vh" p="md">
        <Paper p="xl" withBorder maw={480} w="100%">
          <Stack>
            <Alert color="red" variant="light" title="Could not load your account">
              {check.error.message}
            </Alert>
            <Group>
              <Button onClick={() => void check.refetch()}>Try again</Button>
              {signOutButton}
            </Group>
          </Stack>
        </Paper>
      </Center>
    )
  }

  if (!check.data || !gated) {
    return (
      <Center mih="100vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  return (
    <Center mih="100vh" p="md">
      <Paper p="xl" withBorder maw={480} w="100%">
        <Stack>
          <Title order={2}>Verify your email</Title>
          <Text size="sm">
            This instance needs a verified email. Open the link we sent to <b>{check.data.user.email}</b> to keep using
            Linkdean.
          </Text>
          {resend.isError ? (
            <Alert color="red" variant="light">
              {resend.error.message}
            </Alert>
          ) : null}
          <Group>
            <Button
              leftSection={resend.isSuccess ? <IconCheck {...iconSm} /> : <IconMail {...iconSm} />}
              loading={resend.isPending}
              disabled={resend.isSuccess}
              onClick={() => resend.mutate()}
            >
              {resend.isSuccess ? 'Sent' : 'Resend email'}
            </Button>
            {signOutButton}
          </Group>
        </Stack>
      </Paper>
    </Center>
  )
}
