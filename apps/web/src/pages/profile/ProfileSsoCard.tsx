import { type ReactElement } from 'react'
import { Alert, Badge, Button, Group, Paper, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { IconLink, IconLinkOff, IconShieldLock } from '@tabler/icons-react'

const iconSm = { size: 16, stroke: 1.5 } as const

/** Where Connect sends the browser: the provider, then back to this page. */
export const SSO_CONNECT_URL = '/api/auth/oidc/start?mode=connect'

type Props = {
  connected: boolean
  provider: string | null | undefined
  hasPassword: boolean
  /** Why the last Connect failed, from the callback redirect. */
  error: string | null
  onDismissError: () => void
  onDisconnect: () => void
}

/** Single sign-on state. Disconnect needs a password, so accounts without one cannot lose their only sign-in. */
export function ProfileSsoCard({
  connected,
  provider,
  hasPassword,
  error,
  onDismissError,
  onDisconnect,
}: Props): ReactElement {
  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <Group gap="sm">
          <ThemeIcon size={28} radius="md" variant="light" color="violet">
            <IconShieldLock size={16} stroke={1.5} />
          </ThemeIcon>
          <Title order={4}>Single sign-on</Title>
        </Group>
        {error ? (
          <Alert color="red" variant="light" title="Single sign-on failed" withCloseButton onClose={onDismissError}>
            {error}
          </Alert>
        ) : null}
        {connected ? (
          <>
            <Group gap="xs">
              <Badge color="green" variant="light">
                Connected
              </Badge>
              {provider ? (
                <Text size="sm" c="dimmed">
                  {provider}
                </Text>
              ) : null}
            </Group>
            {hasPassword ? (
              <Group>
                <Button variant="light" color="red" leftSection={<IconLinkOff {...iconSm} />} onClick={onDisconnect}>
                  Disconnect
                </Button>
              </Group>
            ) : (
              <Text size="sm" c="dimmed">
                Single sign-on is this account&apos;s only way to sign in, so it stays connected.
              </Text>
            )}
          </>
        ) : (
          <>
            <Text size="sm" c="dimmed">
              Sign in with your identity provider as well as your password. Your email stays the same.
            </Text>
            <Group>
              <Button component="a" href={SSO_CONNECT_URL} variant="light" leftSection={<IconLink {...iconSm} />}>
                Connect
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Paper>
  )
}
