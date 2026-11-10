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
import {
  LegalFooterLinks,
  LogoMark,
  MASCOT,
  MASCOT_LOGIN_DISPLAY_PX,
  MascotFigure,
  QueryErrorAlert,
  TermsAgreement,
} from '@/components'
import { useBootstrap, useDropSignedOutCache, useLogin, usePendingOidcLink } from '@/hooks'
import { oidcStartUrl, resolveDefaultLanding } from '@/lib'
import motion from '@/styles/motion.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LoginPage(): ReactElement {
  const nav = useNavigate()
  useDropSignedOutCache()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [termsAccepted, setTermsAccepted] = useState(false)
  const bootstrap = useBootstrap()
  const login = useLogin()
  const { oidc_error: oidcError, link } = useSearch({ from: '/login' })
  // Signing in spends the link and clears the cache, so stop asking once it starts.
  const pendingLink = usePendingOidcLink(!!link && !login.isPending && !login.isSuccess)

  useEffect(() => {
    if (bootstrap.data?.needs_bootstrap) {
      void nav({ to: '/onboarding' })
    }
  }, [bootstrap.data, nav])

  async function submit(): Promise<void> {
    if (login.isPending) return
    try {
      const res = await login.mutateAsync({ email, password })
      if (res.sso_connected) {
        notifications.show({ message: 'Single sign-on connected' })
      }
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
  const mailEnabled = bootstrap.data?.mail_enabled ?? false

  return (
    <Group align="flex-start" justify="center" gap={48} mt={80} px="md" pb={48} wrap="nowrap" className={motion.fadeIn}>
      <Stack maw={420} w="100%" gap="lg">
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
            {pendingLink.data && !login.isError ? (
              <Stack gap={4}>
                <Alert color="violet" variant="light">
                  An account already uses {pendingLink.data.email}. Sign in with its password to connect single sign-on.
                </Alert>
                <Text size="sm" c="dimmed">
                  Don&apos;t know this account&apos;s password? Ask your admin.
                </Text>
              </Stack>
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
            {mailEnabled ? (
              <Anchor
                component="button"
                type="button"
                size="sm"
                ta="right"
                mt={-8}
                onClick={() => void nav({ to: '/forgot-password' })}
              >
                Forgot password?
              </Anchor>
            ) : (
              <Text size="sm" c="dimmed" mt={-8}>
                Forgot your password? Ask your admin for a reset link.
              </Text>
            )}
            <Button leftSection={<IconLogin {...iconSm} />} loading={login.isPending} onClick={() => void submit()}>
              Sign in
            </Button>
            {oidc ? (
              <>
                <Divider label="or" labelPosition="center" />
                <TermsAgreement
                  checked={termsAccepted}
                  onChange={setTermsAccepted}
                  description="Needed to create an account with single sign-on."
                />
                <Button
                  component="a"
                  // No href until accepted: the link is neither clickable nor focusable.
                  href={termsAccepted ? oidcStartUrl() : undefined}
                  disabled={!termsAccepted}
                  variant="light"
                  leftSection={<IconLogin {...iconSm} />}
                >
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
        <LegalFooterLinks />
      </Stack>
      <Center visibleFrom="md" mih={420} style={{ flexShrink: 0 }}>
        <MascotFigure src={MASCOT.standingLg} width={MASCOT_LOGIN_DISPLAY_PX} flip />
      </Center>
    </Group>
  )
}
