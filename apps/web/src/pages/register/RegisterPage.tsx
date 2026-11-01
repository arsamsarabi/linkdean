import { useEffect, useRef, useState, type ReactElement } from 'react'
import {
  Anchor,
  Button,
  Center,
  FileButton,
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
import { IconLogin, IconPhoto, IconTrash, IconUserPlus } from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { api } from '@/api'
import { AvatarCropModal, QueryErrorAlert } from '@/components'
import { useBootstrap, useRegister, useUploadAvatar } from '@/hooks'
import { resolveDefaultLanding } from '@/lib'
import { validatePasswordPair } from '@/services'
import motion from '@/styles/motion.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

export function RegisterPage(): ReactElement {
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { invite?: string }
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [inviteCode, setInviteCode] = useState(search.invite ?? '')
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [pendingAvatar, setPendingAvatar] = useState<Blob | null>(null)
  const resetRef = useRef<() => void>(null)
  const bootstrap = useBootstrap()
  const register = useRegister()
  const uploadAvatar = useUploadAvatar()

  useEffect(() => {
    if (search.invite && search.invite !== inviteCode) {
      setInviteCode(search.invite)
    }
  }, [search.invite]) // oxlint-disable-line react-hooks/exhaustive-deps -- sync URL → field once

  useEffect(() => {
    if (bootstrap.data?.needs_bootstrap) {
      void nav({ to: '/onboarding' })
    }
  }, [bootstrap.data, nav])

  async function submit(): Promise<void> {
    const check = validatePasswordPair(password, password2)
    if (!check.ok) {
      notifications.show({ color: 'red', message: check.message })
      return
    }
    try {
      const res = await register.mutateAsync({
        email,
        password,
        display_name: displayName || undefined,
        invite_code: inviteCode.trim() || undefined,
      })
      if (pendingAvatar) {
        try {
          await uploadAvatar.mutateAsync(pendingAvatar)
        } catch (e) {
          notifications.show({ color: 'yellow', message: `Account created; avatar failed: ${e}` })
        }
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
          title="Failed to check registration status"
        />
      </Stack>
    )
  }

  const closed = !bootstrap.data?.needs_bootstrap && bootstrap.data?.registration_open === false
  const hasInvite = inviteCode.trim().length > 0
  if (closed && !hasInvite) {
    return (
      <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
        <Stack gap={6}>
          <Title order={1} c="violet">
            Registration closed
          </Title>
          <Text c="dimmed">
            This instance is invite-only. Ask an admin for an invite link, or paste a code below.
          </Text>
        </Stack>
        <Paper p="xl" radius="md" withBorder>
          <Stack gap="md">
            <TextInput
              label="Invite code"
              placeholder="ldi_…"
              value={inviteCode}
              onChange={(e) => setInviteCode(e.currentTarget.value)}
              data-autofocus
            />
            <Button
              variant="light"
              disabled={!inviteCode.trim()}
              onClick={() => {
                void nav({
                  to: '/register',
                  search: { invite: inviteCode.trim() },
                })
              }}
            >
              Continue with invite
            </Button>
            <Anchor component="button" type="button" onClick={() => void nav({ to: '/login' })}>
              <Group gap={6} justify="center">
                <IconLogin size={14} stroke={1.5} />
                Back to sign in
              </Group>
            </Anchor>
          </Stack>
        </Paper>
      </Stack>
    )
  }

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <Stack gap={6}>
        <Title order={1} c="violet">
          Create account
        </Title>
        <Text c="dimmed">
          {closed ? 'Join with your invite.' : 'Join this Linkdean instance.'}
        </Text>
      </Stack>
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          {closed || hasInvite ? (
            <TextInput
              label="Invite code"
              description={closed ? 'Required for this instance' : 'Optional'}
              value={inviteCode}
              onChange={(e) => setInviteCode(e.currentTarget.value)}
              required={closed}
            />
          ) : null}
          <TextInput label="Display name" value={displayName} onChange={(e) => setDisplayName(e.currentTarget.value)} />
          <TextInput label="Email" required value={email} onChange={(e) => setEmail(e.currentTarget.value)} />
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
            loading={register.isPending}
            onClick={() => void submit()}
            disabled={!email || !password || (closed && !inviteCode.trim())}
          >
            Sign up
          </Button>
          <Anchor component="button" type="button" onClick={() => void nav({ to: '/login' })}>
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
        onCropped={(blob) => {
          setPendingAvatar(blob)
          notifications.show({ message: 'Avatar cropped — will upload on sign up' })
        }}
      />
    </Stack>
  )
}
