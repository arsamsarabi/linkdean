import { useEffect, useState, type ReactElement } from 'react'
import { useNavigate } from '@tanstack/react-router'
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
import { IconLogin, IconRocket, IconUserPlus } from '@tabler/icons-react'
import { LogoMark } from '../../components/logo-mark'
import { QueryErrorAlert } from '../../components/query-error-alert'
import { useLogin } from '../../hooks/auth/useAuth'
import { useBootstrap } from '../../hooks/auth/useBootstrap'
import motion from '../../styles/motion.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LoginPage(): ReactElement {
  const nav = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const bootstrap = useBootstrap()
  const login = useLogin()

  useEffect(() => {
    if (bootstrap.data?.needs_bootstrap) {
      void nav({ to: '/onboarding' })
    }
  }, [bootstrap.data, nav])

  async function submit(): Promise<void> {
    try {
      const res = await login.mutateAsync({ email, password })
      if (!res.user.onboarding_completed) {
        void nav({ to: '/onboarding' })
        return
      }
      void nav({ to: '/library' })
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
          <Button
            leftSection={<IconLogin {...iconSm} />}
            loading={login.isPending}
            onClick={() => void submit()}
          >
            Sign in
          </Button>
          {oidc ? (
            <>
              <Divider label="or" labelPosition="center" />
              <Button
                component="a"
                href="/api/auth/oidc/start"
                variant="light"
                leftSection={<IconLogin {...iconSm} />}
              >
                Sign in with OIDC
              </Button>
            </>
          ) : null}
          <Anchor component="button" type="button" onClick={() => void nav({ to: '/register' })}>
            <Group gap={6} justify="center">
              <IconUserPlus size={14} stroke={1.5} />
              Create an account
            </Group>
          </Anchor>
          <Anchor component="button" type="button" onClick={() => void nav({ to: '/onboarding' })}>
            <Group gap={6} justify="center">
              <IconRocket size={14} stroke={1.5} />
              First install? Open setup
            </Group>
          </Anchor>
        </Stack>
      </Paper>
    </Stack>
  )
}
