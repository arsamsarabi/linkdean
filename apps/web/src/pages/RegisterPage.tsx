import { useEffect, useRef, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import {
  Anchor,
  Button,
  FileButton,
  Group,
  Paper,
  PasswordInput,
  Stack,
  Text,
  TextInput,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLogin, IconPhoto, IconTrash, IconUserPlus } from '@tabler/icons-react'
import { api, setToken } from '../api'
import { AvatarCropModal } from '../components/AvatarCropModal'

const iconSm = { size: 16, stroke: 1.5 } as const

export function RegisterPage() {
  const nav = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [busy, setBusy] = useState(false)
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [pendingAvatar, setPendingAvatar] = useState<Blob | null>(null)
  const resetRef = useRef<() => void>(null)

  useEffect(() => {
    api.bootstrap().then((s) => {
      if (s.needs_bootstrap) nav({ to: '/onboarding' })
    })
  }, [nav])

  async function submit() {
    if (password.length < 8) {
      notifications.show({ color: 'red', message: 'Password min 8 chars' })
      return
    }
    if (password !== password2) {
      notifications.show({ color: 'red', message: 'Passwords mismatch' })
      return
    }
    setBusy(true)
    try {
      const res = await api.register({
        email,
        password,
        display_name: displayName || undefined,
      })
      setToken(res.token)
      if (pendingAvatar) {
        try {
          await api.uploadAvatar(pendingAvatar)
        } catch (e) {
          notifications.show({ color: 'yellow', message: `Account created; avatar failed: ${e}` })
        }
      }
      if (!res.user.onboarding_completed) nav({ to: '/onboarding' })
      else nav({ to: '/library' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className="ld-fade-in">
      <Stack gap={6}>
        <Title order={1} c="violet">
          Create account
        </Title>
        <Text c="dimmed">Join this Linkdean instance.</Text>
      </Stack>
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          <TextInput
            label="Display name"
            value={displayName}
            onChange={(e) => setDisplayName(e.currentTarget.value)}
          />
          <TextInput
            label="Email"
            required
            value={email}
            onChange={(e) => setEmail(e.currentTarget.value)}
          />
          <PasswordInput
            label="Password"
            required
            description="Min 8 characters"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
          />
          <PasswordInput
            label="Confirm password"
            required
            value={password2}
            onChange={(e) => setPassword2(e.currentTarget.value)}
          />
          <Group>
            <FileButton
              resetRef={resetRef}
              accept="image/jpeg,image/png,image/webp,image/gif"
              onChange={(f) => {
                if (f) {
                  setCropFile(f)
                  setCropOpen(true)
                }
              }}
            >
              {(props) => (
                <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />}>
                  {pendingAvatar ? 'Avatar ready — change' : 'Optional avatar'}
                </Button>
              )}
            </FileButton>
            {pendingAvatar ? (
              <Button
                variant="subtle"
                color="red"
                leftSection={<IconTrash {...iconSm} />}
                onClick={() => {
                  setPendingAvatar(null)
                  resetRef.current?.()
                }}
              >
                Clear
              </Button>
            ) : null}
          </Group>
          <Button
            leftSection={<IconUserPlus {...iconSm} />}
            loading={busy}
            onClick={submit}
            disabled={!email || !password}
          >
            Sign up
          </Button>
          <Anchor component="button" type="button" onClick={() => nav({ to: '/login' })}>
            <Group gap={6} justify="center">
              <IconLogin size={14} stroke={1.5} />
              Already have an account? Sign in
            </Group>
          </Anchor>
        </Stack>
      </Paper>

      <AvatarCropModal
        opened={cropOpen}
        file={cropFile}
        onClose={() => {
          setCropOpen(false)
          setCropFile(null)
          resetRef.current?.()
        }}
        onCropped={async (blob) => {
          setPendingAvatar(blob)
          notifications.show({ message: 'Avatar cropped — will upload on sign up' })
        }}
      />
    </Stack>
  )
}
