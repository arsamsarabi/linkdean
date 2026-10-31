import { useEffect, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import {
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
import { api, setToken } from '../api'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LoginPage() {
  const nav = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [oidc, setOidc] = useState(false)
  const [checking, setChecking] = useState(true)

  useEffect(() => {
    api
      .bootstrap()
      .then((s) => {
        setOidc(s.oidc_enabled)
        if (s.needs_bootstrap) nav({ to: '/onboarding' })
      })
      .catch(() => {})
      .finally(() => setChecking(false))
  }, [nav])

  async function submit() {
    setBusy(true)
    try {
      const res = await api.login({ email, password })
      setToken(res.token)
      if (!res.user.onboarding_completed) {
        nav({ to: '/onboarding' })
        return
      }
      nav({ to: '/library' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  if (checking) {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className="ld-fade-in">
      <Stack gap={6}>
        <Group gap="sm" wrap="nowrap">
          <img
            src="/linkdean-symbol.svg"
            alt=""
            width={40}
            height={40}
            style={{ display: 'block' }}
          />
          <Title order={1} c="violet">
            Linkdean
          </Title>
        </Group>
        <Text c="dimmed">Sign in to your bookmarks.</Text>
      </Stack>
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          <TextInput
            label="Email"
            value={email}
            onChange={(e) => setEmail(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
          <PasswordInput
            label="Password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
          <Button leftSection={<IconLogin {...iconSm} />} loading={busy} onClick={submit}>
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
          <Anchor component="button" type="button" onClick={() => nav({ to: '/register' })}>
            <Group gap={6} justify="center">
              <IconUserPlus size={14} stroke={1.5} />
              Create an account
            </Group>
          </Anchor>
          <Anchor component="button" type="button" onClick={() => nav({ to: '/onboarding' })}>
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
