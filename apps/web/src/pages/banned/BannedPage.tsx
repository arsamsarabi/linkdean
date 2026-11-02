import { useEffect, type ReactElement } from 'react'
import { Button, Center, Loader, Paper, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useQuery } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { api, ApiError, banFromBody } from '@/api'
import { useLogout } from '@/hooks'

/** Asks the API whether the ban still holds, so an ended ban leaves this page. */
// Known issue: On a 5xx or network failure this shows a spinner forever with no way to sign out.
// Fix: an error state with retry and Sign out.
export function BannedPage(): ReactElement {
  const nav = useNavigate()
  const logout = useLogout()
  const check = useQuery({ queryKey: ['ban-check'], queryFn: () => api.me(), retry: false })
  const error = check.error instanceof ApiError ? check.error : null
  const ban = banFromBody(error?.body)
  const signedOut = error?.status === 401

  useEffect(() => {
    if (signedOut) void nav({ to: '/login', replace: true })
    else if (check.isSuccess) void nav({ to: '/', replace: true })
  }, [check.isSuccess, nav, signedOut])

  async function signOut(): Promise<void> {
    try {
      await logout.mutateAsync()
    } catch {
      notifications.show({ color: 'red', message: 'Could not sign out. Try again.' })
      return
    }
    await nav({ to: '/login' })
  }

  if (!ban) {
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
          <Title order={2}>Your account is banned</Title>
          <Text size="sm">
            {ban.ends_at ? `This ban ends on ${new Date(ban.ends_at).toLocaleString()}.` : 'This ban is permanent.'}
          </Text>
          <div>
            <Text size="sm" fw={500}>
              Reason
            </Text>
            <Text size="sm">{ban.reason}</Text>
          </div>
          <Button variant="default" loading={logout.isPending} onClick={() => void signOut()}>
            Sign out
          </Button>
        </Stack>
      </Paper>
    </Center>
  )
}
