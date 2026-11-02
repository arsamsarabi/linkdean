import { type ReactElement, type RefObject } from 'react'
import { Anchor, Button, FileButton, Group, Paper, PasswordInput, Stack, Text, TextInput, Title } from '@mantine/core'
import { IconLogin, IconPhoto, IconTrash, IconUserPlus } from '@tabler/icons-react'
import { MIN_SECRET_LENGTH } from '@/services'
import motion from '@/styles/motion.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  closed: boolean
  hasInvite: boolean
  hasShareInvite: boolean
  inviteCode: string
  displayName: string
  email: string
  password: string
  password2: string
  pendingAvatar: Blob | null
  registerPending: boolean
  resetRef: RefObject<(() => void) | null>
  onInviteCode: (v: string) => void
  onDisplayName: (v: string) => void
  onEmail: (v: string) => void
  onPassword: (v: string) => void
  onPassword2: (v: string) => void
  onPickAvatar: (f: File | null) => void
  onClearAvatar: () => void
  onSubmit: () => void
  onLogin: () => void
}

export function RegisterForm({
  closed,
  hasInvite,
  hasShareInvite,
  inviteCode,
  displayName,
  email,
  password,
  password2,
  pendingAvatar,
  registerPending,
  resetRef,
  onInviteCode,
  onDisplayName,
  onEmail,
  onPassword,
  onPassword2,
  onPickAvatar,
  onClearAvatar,
  onSubmit,
  onLogin,
}: Props): ReactElement {
  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <Stack gap={6}>
        <Title order={1} c="violet">
          Create account
        </Title>
        <Text c="dimmed">{closed ? 'Join with your invite.' : 'Join this Linkdean instance.'}</Text>
        {hasShareInvite ? (
          <Text size="sm" c="dimmed">
            Use the email address you were invited at, and the invite will be waiting for you.
          </Text>
        ) : null}
      </Stack>
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          {closed || hasInvite ? (
            <TextInput
              label="Invite code"
              description={closed ? 'Required for this instance' : 'Optional'}
              value={inviteCode}
              onChange={(e) => onInviteCode(e.currentTarget.value)}
              required={closed}
            />
          ) : null}
          <TextInput label="Display name" value={displayName} onChange={(e) => onDisplayName(e.currentTarget.value)} />
          <TextInput label="Email" required value={email} onChange={(e) => onEmail(e.currentTarget.value)} />
          <PasswordInput
            label="Password"
            required
            description={`Min ${MIN_SECRET_LENGTH} characters`}
            value={password}
            onChange={(e) => onPassword(e.currentTarget.value)}
          />
          <PasswordInput
            label="Confirm password"
            required
            value={password2}
            onChange={(e) => onPassword2(e.currentTarget.value)}
          />
          <Group>
            <FileButton resetRef={resetRef} accept="image/jpeg,image/png,image/webp,image/gif" onChange={onPickAvatar}>
              {(props) => (
                <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />}>
                  {pendingAvatar ? 'Avatar ready — change' : 'Optional avatar'}
                </Button>
              )}
            </FileButton>
            {pendingAvatar ? (
              <Button variant="subtle" color="red" leftSection={<IconTrash {...iconSm} />} onClick={onClearAvatar}>
                Clear
              </Button>
            ) : null}
          </Group>
          <Button
            leftSection={<IconUserPlus {...iconSm} />}
            loading={registerPending}
            onClick={onSubmit}
            disabled={!email || !password || (closed && !inviteCode.trim())}
          >
            Sign up
          </Button>
          <Anchor component="button" type="button" onClick={onLogin}>
            <Group gap={6} justify="center">
              <IconLogin size={14} stroke={1.5} />
              Already have an account? Sign in
            </Group>
          </Anchor>
        </Stack>
      </Paper>
    </Stack>
  )
}
