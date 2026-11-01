import { useState, type ReactElement } from 'react'
import { Center, Loader, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useNavigate, useParams } from '@tanstack/react-router'
import { FollowListModal, ProfileBookmarksList, ProfileHeader, type FollowListKind } from '@/components'
import { useFollowUser, useProfile, useUnfollowUser } from '@/hooks'

export function UserProfileView({ userId, onEdit }: { userId: string; onEdit: () => void }): ReactElement {
  const profile = useProfile(userId)
  const follow = useFollowUser()
  const unfollow = useUnfollowUser()
  const [list, setList] = useState<FollowListKind | null>(null)

  if (profile.isLoading) {
    return (
      <Center py="xl">
        <Loader size="sm" />
      </Center>
    )
  }
  if (!profile.data) {
    return (
      <Stack gap="xs">
        <Title order={3}>Profile not available</Title>
        <Text c="dimmed">This link may be out of date.</Text>
      </Stack>
    )
  }

  const data = profile.data
  const pending = follow.isPending || unfollow.isPending

  async function toggleFollow(): Promise<void> {
    try {
      await (data.follow.following ? unfollow : follow).mutateAsync(data.id)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Stack gap="xl">
      <ProfileHeader
        profile={data}
        followPending={pending}
        onToggleFollow={() => void toggleFollow()}
        onEdit={onEdit}
        onOpenList={setList}
      />
      <ProfileBookmarksList userId={data.id} emptyLabel="Nothing to show yet." />
      <FollowListModal userId={data.id} kind={list} onClose={() => setList(null)} />
    </Stack>
  )
}

export function UserProfilePage(): ReactElement {
  const { id } = useParams({ from: '/_app/u/$id' })
  const navigate = useNavigate()
  return <UserProfileView userId={id} onEdit={() => void navigate({ to: '/profile' })} />
}
