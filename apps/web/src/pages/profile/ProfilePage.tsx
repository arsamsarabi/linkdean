import { useEffect, useRef, useState, type ReactElement } from 'react'
import { Group, Skeleton, Stack, ThemeIcon, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconUser } from '@tabler/icons-react'
import { AvatarCropModal, QueryErrorAlert } from '@/components'
import { useChangePassword, useDeleteAvatar, useMe, useUpdateProfile, useUploadAvatar } from '@/hooks'
import { validatePasswordPair } from '@/services'
import { ProfileAboutCard } from './ProfileAboutCard'
import { ProfileBlocksCard } from './ProfileBlocksCard'
import { ProfileFollowsCard } from './ProfileFollowsCard'
import { ProfilePasswordCard } from './ProfilePasswordCard'

function ProfileSkeleton(): ReactElement {
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

export function ProfilePage(): ReactElement {
  const me = useMe()
  const user = me.data?.user
  const saveProfile = useUpdateProfile()
  const changePw = useChangePassword()
  const uploadAvatar = useUploadAvatar()
  const deleteAvatar = useDeleteAvatar()

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

  async function onCropped(blob: Blob): Promise<void> {
    try {
      await uploadAvatar.mutateAsync(blob)
      setAvatarBust(Date.now())
      notifications.show({ message: 'Avatar updated' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const label = user?.display_name || user?.email || '?'

  return (
    <Stack gap="xl" maw={560}>
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="violet">
          <IconUser size={22} stroke={1.5} />
        </ThemeIcon>
        <Title order={2}>Profile</Title>
      </Group>

      {me.isError ? <QueryErrorAlert error={me.error} onRetry={() => void me.refetch()} /> : null}

      {!user ? (
        <ProfileSkeleton />
      ) : (
        <>
          <ProfileAboutCard
            label={label}
            hasAvatar={!!user.has_avatar}
            avatarBust={avatarBust}
            displayName={displayName}
            bio={bio}
            timezone={timezone}
            email={email}
            saving={saveProfile.isPending}
            resetRef={resetRef}
            onDisplayName={setDisplayName}
            onBio={setBio}
            onTimezone={setTimezone}
            onEmail={setEmail}
            onPickAvatar={(f) => {
              if (f) {
                setCropFile(f)
                setCropOpen(true)
              }
            }}
            onRemoveAvatar={() => {
              void deleteAvatar
                .mutateAsync()
                .then(() => {
                  setAvatarBust(Date.now())
                  notifications.show({ message: 'Avatar removed' })
                })
                .catch((e) => notifications.show({ color: 'red', message: String(e) }))
            }}
            onSave={() => {
              saveProfile.mutate(
                {
                  display_name: displayName || null,
                  bio: bio || null,
                  timezone: timezone || null,
                  email,
                },
                {
                  onSuccess: () => notifications.show({ message: 'Profile saved' }),
                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                }
              )
            }}
          />

          <ProfileFollowsCard />

          <ProfileBlocksCard />

          <ProfilePasswordCard
            hasPassword={!!user.has_password}
            currentPw={currentPw}
            newPw={newPw}
            newPw2={newPw2}
            saving={changePw.isPending}
            onCurrentPw={setCurrentPw}
            onNewPw={setNewPw}
            onNewPw2={setNewPw2}
            onSubmit={() => {
              const check = validatePasswordPair(newPw, newPw2)
              if (!check.ok) {
                notifications.show({ color: 'red', message: check.message })
                return
              }
              changePw.mutate(
                { current_password: currentPw, new_password: newPw },
                {
                  onSuccess: () => {
                    setCurrentPw('')
                    setNewPw('')
                    setNewPw2('')
                    notifications.show({ message: 'Password updated' })
                  },
                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                }
              )
            }}
          />
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
