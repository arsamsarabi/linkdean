import { useEffect, useRef, useState, type ReactElement } from 'react'
import { Group, Skeleton, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconUser } from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import {
  AvatarCropModal,
  FollowListModal,
  ProfileBookmarksList,
  ProfileHeader,
  QueryErrorAlert,
  type FollowListKind,
} from '@/components'
import {
  useBootstrap,
  useChangePassword,
  useDeleteAvatar,
  useMe,
  useProfile,
  useSyncDuringRender,
  useUpdateProfile,
  useUploadAvatar,
} from '@/hooks'
import type { ProfileVisibility } from '@/schemas'
import { validatePasswordPair } from '@/services'
import { ChangeEmailModal } from './ChangeEmailModal'
import { DisconnectSsoModal } from './DisconnectSsoModal'
import { ProfileAboutCard } from './ProfileAboutCard'
import { ProfileBlocksCard } from './ProfileBlocksCard'
import { ProfileDeclinedInvitesCard } from './ProfileDeclinedInvitesCard'
import { ProfileEditDrawer } from './ProfileEditDrawer'
import { ProfilePasswordCard } from './ProfilePasswordCard'
import { ProfileSsoCard } from './ProfileSsoCard'

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

  const profile = useProfile(user?.id ?? '', !!user)
  const bootstrap = useBootstrap(!!user).data
  const mailEnabled = bootstrap?.mail_enabled ?? false
  const oidcEnabled = bootstrap?.oidc_enabled ?? false
  const [changeEmailOpen, setChangeEmailOpen] = useState(false)
  const [disconnectOpen, setDisconnectOpen] = useState(false)

  // Back from a Connect: the callback says how it went. Keep the outcome, then clean the URL.
  const nav = useNavigate()
  const search = useSearch({ from: '/_app/profile' })
  const [ssoError, setSsoError] = useState<string | null>(search.oidc_error ?? null)
  const ssoReturnHandled = useRef(false)
  useEffect(() => {
    if (ssoReturnHandled.current || (!search.sso && !search.oidc_error)) return
    ssoReturnHandled.current = true
    if (search.sso === 'connected') notifications.show({ message: 'Single sign-on connected' })
    void nav({ to: '/profile', search: {}, replace: true })
  }, [search.sso, search.oidc_error, nav])

  const [editOpen, setEditOpen] = useState(() => !!search.sso || !!search.oidc_error)
  const [list, setList] = useState<FollowListKind | null>(null)
  const [visibility, setVisibility] = useState<ProfileVisibility>('hidden')
  const [displayName, setDisplayName] = useState('')
  const [bio, setBio] = useState('')
  const [timezone, setTimezone] = useState<string | null>(null)
  const [currentPw, setCurrentPw] = useState('')
  const [revokeKeys, setRevokeKeys] = useState(false)
  const [newPw, setNewPw] = useState('')
  const [newPw2, setNewPw2] = useState('')
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [avatarBust, setAvatarBust] = useState(0)
  const resetRef = useRef<() => void>(null)

  function resetFields(): void {
    if (!user) return
    setDisplayName(user.display_name || '')
    setBio(user.bio || '')
    setTimezone(user.timezone || null)
    setVisibility(user.profile_visibility)
  }

  useSyncDuringRender([user], resetFields)

  function closeEditor(): void {
    resetFields()
    setCurrentPw('')
    setNewPw('')
    setNewPw2('')
    setRevokeKeys(false)
    setEditOpen(false)
  }

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
    <Stack gap="xl">
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
          {profile.data ? (
            <ProfileHeader
              profile={profile.data}
              avatarBust={avatarBust}
              onToggleFollow={() => undefined}
              onEdit={() => setEditOpen(true)}
              onOpenList={setList}
            />
          ) : null}

          {user.profile_visibility === 'hidden' ? (
            <Text size="sm" c="dimmed">
              Your profile is hidden. Other people can't open it, including followers.
            </Text>
          ) : null}

          <Stack gap="xs">
            <Title order={4}>What others can see</Title>
            <ProfileBookmarksList
              userId={user.id}
              emptyLabel="Nothing yet. Set a category to Followers or Members to show its bookmarks here."
            />
          </Stack>

          <ProfileBlocksCard />
          <ProfileDeclinedInvitesCard />

          <FollowListModal userId={user.id} kind={list} onClose={() => setList(null)} />

          <ProfileEditDrawer
            opened={editOpen}
            visibility={visibility}
            onVisibility={setVisibility}
            onClose={closeEditor}
            dirty={
              displayName !== (user.display_name || '') ||
              bio !== (user.bio || '') ||
              timezone !== (user.timezone || null) ||
              visibility !== user.profile_visibility ||
              !!(currentPw || newPw || newPw2)
            }
          >
            <ProfileAboutCard
              label={label}
              hasAvatar={!!user.has_avatar}
              avatarBust={avatarBust}
              displayName={displayName}
              bio={bio}
              timezone={timezone}
              email={user.email}
              hasPassword={!!user.has_password}
              mailEnabled={mailEnabled}
              saving={saveProfile.isPending}
              resetRef={resetRef}
              onDisplayName={setDisplayName}
              onBio={setBio}
              onTimezone={setTimezone}
              onChangeEmail={() => setChangeEmailOpen(true)}
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
                    profile_visibility: visibility,
                  },
                  {
                    onSuccess: () => {
                      setEditOpen(false)
                      notifications.show({ message: 'Profile saved' })
                    },
                    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                  }
                )
              }}
            />

            <ProfilePasswordCard
              hasPassword={!!user.has_password}
              currentPw={currentPw}
              newPw={newPw}
              newPw2={newPw2}
              revokeKeys={revokeKeys}
              onRevokeKeys={setRevokeKeys}
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
                  { current_password: currentPw, new_password: newPw, revoke_api_keys: revokeKeys },
                  {
                    onSuccess: () => {
                      setCurrentPw('')
                      setNewPw('')
                      setNewPw2('')
                      setRevokeKeys(false)
                      notifications.show({ message: 'Password updated' })
                    },
                    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                  }
                )
              }}
            />

            {oidcEnabled ? (
              <ProfileSsoCard
                connected={!!user.sso_connected}
                provider={user.sso_provider}
                hasPassword={!!user.has_password}
                error={ssoError}
                onDismissError={() => setSsoError(null)}
                onDisconnect={() => setDisconnectOpen(true)}
              />
            ) : null}
          </ProfileEditDrawer>
        </>
      )}

      {changeEmailOpen && user ? (
        <ChangeEmailModal currentEmail={user.email} onClose={() => setChangeEmailOpen(false)} />
      ) : null}

      {disconnectOpen ? <DisconnectSsoModal onClose={() => setDisconnectOpen(false)} /> : null}

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
