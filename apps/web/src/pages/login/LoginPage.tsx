import { useEffect, useState, type ReactElement } from 'react'
import {
  Alert,
  Anchor,
  Button,
  Center,
  Divider,
  Group,
  Loader,
  Paper,
  PasswordInput,
  Stack,
  Text,
  TextInput,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLogin, IconUserPlus } from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { api } from '@/api'
import { LogoMark, QueryErrorAlert } from '@/components'
import { useBootstrap, useLogin } from '@/hooks'
import { resolveDefaultLanding } from '@/lib'
import motion from '@/styles/motion.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LoginPage(): ReactElement {
  const nav = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const bootstrap = useBootstrap()
  const login = useLogin()
  const { oidc_error: oidcError } = useSearch({ from: '/login' })

  useEffect(() => {
    if (bootstrap.data?.needs_bootstrap) {
      void nav({ to: '/onboarding' })
    }
  }, [bootstrap.data, nav])

  async function submit(): Promise<void> {
    if (login.isPending) return
    try {
      const res = await login.mutateAsync({ email, password })
      if (!res.user.onboarding_completed) {
        void nav({ to: '/onboarding' })
        return
      }
      const { settings } = await api.getSettings()
      void nav({ to: resolveDefaultLanding(settings.default_landing) })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  if (bootstrap.isLoading) {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  if (bootstrap.isError) {
    return (
      <Stack maw={420} mx="auto" mt={80} px="md">
        <QueryErrorAlert
          error={bootstrap.error}
          onRetry={() => void bootstrap.refetch()}
          title="Failed to check bootstrap status"
        />
      </Stack>
    )
  }

  const oidc = bootstrap.data?.oidc_enabled ?? false
  const registrationOpen = bootstrap.data?.registration_open ?? true

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <Stack gap={6}>
        <Group gap="sm" wrap="nowrap">
          <LogoMark size={40} />
          <Title order={1} c="violet">
            Linkdean
          </Title>
        </Group>
        <Text c="dimmed">Sign in to your bookmarks.</Text>
      </Stack>
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          {oidcError && !login.isError ? (
            <Alert color="red" variant="light" title="Single sign-on failed">
              {oidcError}
            </Alert>
          ) : null}
          {login.isError ? (
            <Alert color="red" variant="light">
              {String(login.error)}
            </Alert>
          ) : null}
          <TextInput
            label="Email"
            value={email}
            onChange={(e) => setEmail(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && void submit()}
          />
          <PasswordInput
            label="Password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && void submit()}
          />
          <Button leftSection={<IconLogin {...iconSm} />} loading={login.isPending} onClick={() => void submit()}>
            Sign in
          </Button>
          {oidc ? (
            <>
              <Divider label="or" labelPosition="center" />
              <Button component="a" href="/api/auth/oidc/start" variant="light" leftSection={<IconLogin {...iconSm} />}>
                Sign in with OIDC
              </Button>
            </>
          ) : null}
          {registrationOpen ? (
            <Anchor component="button" type="button" onClick={() => void nav({ to: '/register' })}>
              <Group gap={6} justify="center">
                <IconUserPlus size={14} stroke={1.5} />
                Create an account
              </Group>
            </Anchor>
          ) : null}
        </Stack>
      </Paper>
    </Stack>
  )
}
