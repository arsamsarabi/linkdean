import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  Divider,
  Fieldset,
  FileButton,
  Group,
  Paper,
  PasswordInput,
  Select,
  Skeleton,
  Stack,
  Textarea,
  TextInput,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconDeviceFloppy,
  IconInfoCircle,
  IconKey,
  IconPhoto,
  IconTrash,
  IconUser,
} from '@tabler/icons-react'
import { api } from '../api'
import { AvatarCropModal } from '../components/AvatarCropModal'
import { UserAvatar } from '../components/UserAvatar'

const iconSm = { size: 16, stroke: 1.5 } as const

const TIMEZONES = Intl.supportedValuesOf
  ? Intl.supportedValuesOf('timeZone')
  : ['UTC', 'Europe/Amsterdam', 'America/New_York', 'Asia/Tokyo']

function ProfileSkeleton() {
  return (
    <Stack gap="lg" maw={560}>
      <Group>
        <Skeleton circle height={80} width={80} />
        <Skeleton height={36} width={140} />
      </Group>
      <Skeleton height={36} />
      <Skeleton height={72} />
      <Skeleton height={36} />
      <Skeleton height={36} />
    </Stack>
  )
}

export function ProfilePage() {
  const qc = useQueryClient()
  const me = useQuery({ queryKey: ['me'], queryFn: api.me })
  const user = me.data?.user

  const [displayName, setDisplayName] = useState('')
  const [bio, setBio] = useState('')
  const [timezone, setTimezone] = useState<string | null>(null)
  const [email, setEmail] = useState('')
  const [currentPw, setCurrentPw] = useState('')
  const [newPw, setNewPw] = useState('')
  const [newPw2, setNewPw2] = useState('')
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [avatarBust, setAvatarBust] = useState(0)
  const resetRef = useRef<() => void>(null)

  useEffect(() => {
    if (!user) return
    setDisplayName(user.display_name || '')
    setBio(user.bio || '')
    setTimezone(user.timezone || null)
    setEmail(user.email)
  }, [user])

  const saveProfile = useMutation({
    mutationFn: () =>
      api.updateProfile({
        display_name: displayName || null,
        bio: bio || null,
        timezone: timezone || null,
        email,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['me'] })
      notifications.show({ message: 'Profile saved' })
    },
    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
  })

  const changePw = useMutation({
    mutationFn: () =>
      api.changePassword({ current_password: currentPw, new_password: newPw }),
    onSuccess: () => {
      setCurrentPw('')
      setNewPw('')
      setNewPw2('')
      notifications.show({ message: 'Password updated' })
    },
    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
  })

  async function onCropped(blob: Blob) {
    try {
      await api.uploadAvatar(blob)
      setAvatarBust(Date.now())
      qc.invalidateQueries({ queryKey: ['me'] })
      notifications.show({ message: 'Avatar updated' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function removeAvatar() {
    try {
      await api.deleteAvatar()
      setAvatarBust(Date.now())
      qc.invalidateQueries({ queryKey: ['me'] })
      notifications.show({ message: 'Avatar removed' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const label = user?.display_name || user?.email || '?'

  return (
    <Stack gap="xl" maw={560}>
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="harbor">
          <IconUser size={22} stroke={1.5} />
        </ThemeIcon>
        <Title order={2}>Profile</Title>
      </Group>

      {!user ? (
        <ProfileSkeleton />
      ) : (
        <>
          <Paper p="xl" withBorder>
            <Stack gap="lg">
              <Group gap="lg" align="flex-start">
                <UserAvatar
                  hasAvatar={!!user.has_avatar}
                  label={label}
                  size={80}
                  bust={avatarBust}
                />
                <Stack gap="xs">
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
                        Upload avatar
                      </Button>
                    )}
                  </FileButton>
                  {user.has_avatar && (
                    <Button
                      variant="subtle"
                      color="red"
                      leftSection={<IconTrash {...iconSm} />}
                      onClick={removeAvatar}
                    >
                      Remove
                    </Button>
                  )}
                </Stack>
              </Group>

              <Divider />

              <Fieldset legend="About you" variant="unstyled">
                <Stack gap="md">
                  <TextInput
                    label="Display name"
                    value={displayName}
                    onChange={(e) => setDisplayName(e.currentTarget.value)}
                  />
                  <Textarea
                    label="Bio"
                    value={bio}
                    onChange={(e) => setBio(e.currentTarget.value)}
                    minRows={2}
                    autosize
                  />
                  <Select
                    label="Timezone"
                    searchable
                    clearable
                    data={TIMEZONES}
                    value={timezone}
                    onChange={setTimezone}
                  />
                  <TextInput
                    label="Email"
                    value={email}
                    onChange={(e) => setEmail(e.currentTarget.value)}
                  />
                </Stack>
              </Fieldset>

              <Button
                leftSection={<IconDeviceFloppy {...iconSm} />}
                loading={saveProfile.isPending}
                onClick={() => saveProfile.mutate()}
              >
                Save profile
              </Button>
            </Stack>
          </Paper>

          {user.has_password ? (
            <Paper p="xl" withBorder>
              <Stack gap="lg">
                <Group gap="sm">
                  <ThemeIcon size={28} radius="md" variant="light" color="harbor">
                    <IconKey size={16} stroke={1.5} />
                  </ThemeIcon>
                  <Title order={4}>Change password</Title>
                </Group>
                <PasswordInput
                  label="Current password"
                  value={currentPw}
                  onChange={(e) => setCurrentPw(e.currentTarget.value)}
                />
                <PasswordInput
                  label="New password"
                  description="Min 8 characters"
                  value={newPw}
                  onChange={(e) => setNewPw(e.currentTarget.value)}
                />
                <PasswordInput
                  label="Confirm new password"
                  value={newPw2}
                  onChange={(e) => setNewPw2(e.currentTarget.value)}
                />
                <Button
                  leftSection={<IconKey {...iconSm} />}
                  loading={changePw.isPending}
                  disabled={newPw.length < 8 || newPw !== newPw2 || !currentPw}
                  onClick={() => {
                    if (newPw !== newPw2) {
                      notifications.show({ color: 'red', message: 'Passwords mismatch' })
                      return
                    }
                    changePw.mutate()
                  }}
                >
                  Update password
                </Button>
              </Stack>
            </Paper>
          ) : (
            <Alert icon={<IconInfoCircle size={16} />} color="harbor" variant="light">
              OIDC account — no local password to change.
            </Alert>
          )}
        </>
      )}

      <AvatarCropModal
        opened={cropOpen}
        file={cropFile}
        onClose={() => {
          setCropOpen(false)
          setCropFile(null)
          resetRef.current?.()
        }}
        onCropped={onCropped}
      />
    </Stack>
  )
}
